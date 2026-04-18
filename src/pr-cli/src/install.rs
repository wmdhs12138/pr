use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::color::*;
use crate::plugin::load_plugins;
use crate::shared::{
    get_download_cache_dir, get_installed_rootfs_dir,
    get_native_busybox, get_native_proot, get_native_bash, get_native_loader,
    get_plugins_dir, get_prefix, msg_error, msg_status, DEFAULT_FAKE_KERNEL_RELEASE,
    DEFAULT_FAKE_KERNEL_VERSION, DEFAULT_PRIMARY_NAMESERVER, DEFAULT_SECONDARY_NAMESERVER,
};

fn detect_device_arch() -> String {
    if let Ok(arch) = std::env::var("DISTRO_ARCH") {
        if !arch.is_empty() {
            return arch;
        }
    }

    let prefix = get_prefix();
    let bin_path = format!("{}/bin/busybox", prefix);
    let path = if Path::new(&bin_path).exists() {
        &bin_path
    } else {
        "/system/bin/sh"
    };

    match std::fs::read(path) {
        Ok(data) => {
            if data.len() > 20 && &data[1..4] == b"ELF" {
                let machine = &data[18..20];
                match machine {
                    [0xb7, _] => "aarch64".to_string(),
                    [0x28, _] => "arm".to_string(),
                    [0x3e, _] => "x86_64".to_string(),
                    [0x03, _] => "i686".to_string(),
                    [0xf3, _] => "riscv64".to_string(),
                    [0x08, _] => "mips".to_string(),
                    _ => "unknown".to_string(),
                }
            } else {
                "unknown".to_string()
            }
        }
        Err(_) => "unknown".to_string(),
    }
}

fn run_cmd(bin: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| format!("failed to execute {}: {}", bin, e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("{} exited with code {:?}", bin, output.status.code())
        } else {
            stderr
        })
    }
}

fn run_busybox_cmd(applet: &str, args: &[&str]) -> Result<String, String> {
    let busybox = get_native_busybox();
    let mut full_args = vec![applet.to_string()];
    full_args.extend(args.iter().map(|s| s.to_string()));
    let output = Command::new(&busybox)
        .arg0("busybox")
        .args(&full_args)
        .output()
        .map_err(|e| format!("failed to execute busybox {}: {}", applet, e))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            format!("busybox {} exited with code {:?}", applet, output.status.code())
        } else {
            stderr
        })
    }
}

fn extract_tarball(
    archive_path: &str,
    dest: &str,
    strip_components: usize,
    exclude: &[&str],
) -> Result<(), String> {
    let file = fs::File::open(archive_path)
        .map_err(|e| format!("open archive: {}", e))?;
    let decompressor = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(decompressor);
    archive.set_preserve_permissions(true);
    archive.set_preserve_mtime(true);

    for entry in archive.entries().map_err(|e| format!("read tar entries: {}", e))? {
        let mut entry = entry.map_err(|e| format!("read tar entry: {}", e))?;
        let path = entry.path().map_err(|e| format!("get tar path: {}", e))?;
        let path_str = path.to_string_lossy();

        if exclude.iter().any(|exc| path_str.starts_with(exc) || path_str.starts_with(&format!("./{}", exc))) {
            continue;
        }

        let stripped = if strip_components > 0 {
            match path.components().skip(strip_components).collect::<std::path::PathBuf>().as_path().to_str() {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => continue,
            }
        } else {
            path_str.to_string()
        };

        let dest_path = format!("{}/{}", dest, stripped);
        let dest_path = std::path::Path::new(&dest_path);

        if let Some(parent) = dest_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let entry_type = entry.header().entry_type();
        if entry_type.is_dir() {
            fs::create_dir_all(dest_path)
                .map_err(|e| format!("create dir {}: {}", dest_path.display(), e))?;
            let mode = entry.header().mode().unwrap_or(0o755);
            let _ = fs::set_permissions(dest_path, fs::Permissions::from_mode(mode));
        } else if entry_type.is_symlink() {
            let target = entry.link_name().map_err(|e| format!("read symlink target: {}", e))?;
            let target = target.map(|t| t.to_string_lossy().to_string()).unwrap_or_default();
            if dest_path.exists() {
                let _ = fs::remove_file(dest_path);
            }
            std::os::unix::fs::symlink(&target, dest_path)
                .map_err(|e| format!("symlink {} -> {}: {}", dest_path.display(), target, e))?;
        } else if entry_type.is_hard_link() {
            let target = entry.link_name().map_err(|e| format!("read hardlink target: {}", e))?;
            let target_str = target.map(|t| t.to_string_lossy().to_string()).unwrap_or_default();
            let link_target = if strip_components > 0 {
                format!("{}/{}", dest, {
                    let p = std::path::Path::new(&target_str);
                    match p.components().skip(strip_components).collect::<std::path::PathBuf>().as_path().to_str() {
                        Some(s) if !s.is_empty() => s.to_string(),
                        _ => continue,
                    }
                })
            } else {
                format!("{}/{}", dest, target_str)
            };
            if dest_path.exists() {
                let _ = fs::remove_file(dest_path);
            }
            let _ = fs::hard_link(&link_target, dest_path);
        } else {
            let mut out = fs::File::create(dest_path)
                .map_err(|e| format!("create {}: {}", dest_path.display(), e))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("write {}: {}", dest_path.display(), e))?;
            let mode = entry.header().mode().unwrap_or(0o644);
            let _ = fs::set_permissions(dest_path, fs::Permissions::from_mode(mode));
        }
    }

    Ok(())
}

fn download_file(url: &str, output_path: &str, max_retries: u32) -> Result<(), String> {
    let mut retry = 0;
    let mut delay = 5u64;

    while retry < max_retries {
        if retry > 0 {
            println!(
                "{}[{}*{}{}] Retry {}/{} after {}s...{}",
                BLUE, YELLOW, BLUE, CYAN, retry, max_retries, delay, RESET
            );
            std::thread::sleep(std::time::Duration::from_secs(delay));
            delay = (delay * 2).min(60);
        }

        let _ = fs::remove_file(output_path);

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("create tokio runtime: {}", e))?;

        let result = rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(30))
                .danger_accept_invalid_certs(true)
                .build()
                .map_err(|e| format!("build HTTP client: {}", e))?;

            let resp = client.get(url).send().await.map_err(|e| {
                let mut msg = format!("HTTP request failed: {}", e);
                let mut source: Option<&dyn Error> = e.source();
                while let Some(err) = source {
                    msg.push_str(&format!("\n  caused by: {}", err));
                    source = err.source();
                }
                eprintln!("{}", msg);
                msg
            })?;

            if !resp.status().is_success() {
                return Err(format!("HTTP {}", resp.status()));
            }

            let total = resp.content_length();
            let mut file = fs::File::create(output_path)
                .map_err(|e| format!("create file: {}", e))?;

            let mut downloaded: u64 = 0;
            let mut stream = resp.bytes_stream();
            use futures_util::StreamExt;

            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| format!("read chunk: {}", e))?;
                file.write_all(&chunk).map_err(|e| format!("write chunk: {}", e))?;
                downloaded += chunk.len() as u64;

                if let Some(total) = total {
                    if total > 0 {
                        let pct = (downloaded as f64 / total as f64 * 100.0) as u32;
                        println!("\r{}[{}*{}{}] {:.1}MB / {:.1}MB ({}%){}   ",
                            BLUE, GREEN, BLUE, CYAN,
                            downloaded as f64 / (1024.0 * 1024.0),
                            total as f64 / (1024.0 * 1024.0),
                            pct, RESET);
                    }
                }
            }

            file.sync_all().map_err(|e| format!("flush file: {}", e))?;
            drop(file);

            Ok(())
        });

        match result {
            Ok(()) => {
                if Path::new(output_path).exists()
                    && fs::metadata(output_path).map(|m| m.len()).unwrap_or(0) > 0
                {
                    return Ok(());
                }
            }
            Err(e) => {
                eprintln!("download error: {}", e);
            }
        }

        retry += 1;
    }

    let _ = fs::remove_file(output_path);
    Err(format!("download failed after {} retries", max_retries))
}

fn verify_sha256(expected: &str, filepath: &str) -> Result<(), String> {
    let mut file = fs::File::open(filepath)
        .map_err(|e| format!("open {}: {}", filepath, e))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("read {}: {}", filepath, e))?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();
    let actual = format!("{:x}", hash);

    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "SHA256 mismatch: expected {}, got {}",
            expected, actual
        ))
    }
}

fn setup_fake_sysdata(rootfs: &str) -> Result<(), String> {
    for d in &["proc", "sys", "sys/.empty"] {
        let path = format!("{}/{}", rootfs, d);
        fs::create_dir_all(&path).map_err(|e| format!("failed to create {}: {}", path, e))?;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o700));
    }

    let loadavg = "0.12 0.07 0.02 2/165 765\n";
    fs::write(format!("{}/proc/.loadavg", rootfs), loadavg)
        .map_err(|e| format!("write .loadavg: {}", e))?;

    let stat = "cpu  1957 0 2877 93280 262 342 254 87 0 0\ncpu0 31 0 226 12027 82 10 4 9 0 0\ncpu1 45 0 664 11144 21 263 233 12 0 0\ncpu2 494 0 537 11283 27 10 3 8 0 0\ncpu3 359 0 234 11723 24 26 5 7 0 0\ncpu4 295 0 268 11772 10 12 2 12 0 0\ncpu5 270 0 251 11833 15 3 1 10 0 0\ncpu6 430 0 520 11386 30 8 1 12 0 0\ncpu7 30 0 172 12108 50 8 1 13 0 0\nintr 127541\nctxt 140223\nbtime 1680020856\nprocesses 772\nprocs_running 2\nprocs_blocked 0\nsoftirq 75663 0 5903 6 25375 10774 0 243 11685 0 21677\n";
    fs::write(format!("{}/proc/.stat", rootfs), stat).map_err(|e| format!("write .stat: {}", e))?;

    let uptime = "124.08 932.80\n";
    fs::write(format!("{}/proc/.uptime", rootfs), uptime)
        .map_err(|e| format!("write .uptime: {}", e))?;

    let version = format!(
        "Linux version {} (proot@pr) (gcc (GCC) 13.3.0, GNU ld (GNU Binutils) 2.42) {}\n",
        DEFAULT_FAKE_KERNEL_RELEASE, DEFAULT_FAKE_KERNEL_VERSION
    );
    fs::write(format!("{}/proc/.version", rootfs), version)
        .map_err(|e| format!("write .version: {}", e))?;

    let vmstat = "nr_free_pages 1743136\nnr_zone_inactive_anon 179281\nnr_zone_active_anon 7183\nnr_zone_inactive_file 22858\nnr_zone_active_file 51328\nnr_zone_unevictable 642\nnr_zone_write_pending 0\nnr_mlock 0\nnr_bounce 0\n";
    fs::write(format!("{}/proc/.vmstat", rootfs), vmstat)
        .map_err(|e| format!("write .vmstat: {}", e))?;

    Ok(())
}

fn write_config_files(rootfs: &str, distro_name: &str) -> Result<(), String> {
    let prefix = get_prefix();
    let default_path_env = format!(
        "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/local/games:/usr/games:{}/bin",
        prefix
    );

    // /etc/resolv.conf
    let resolv_path = format!("{}/etc/resolv.conf", rootfs);
    let _ = fs::remove_file(&resolv_path);
    let resolv = format!(
        "nameserver {}\nnameserver {}\n",
        DEFAULT_PRIMARY_NAMESERVER, DEFAULT_SECONDARY_NAMESERVER
    );
    fs::write(&resolv_path, &resolv).map_err(|e| format!("write resolv.conf: {}", e))?;
    msg_status(&format!("Creating file '{}'...", resolv_path));

    // /etc/hosts
    let hosts_path = format!("{}/etc/hosts", rootfs);
    let _ = fs::set_permissions(&hosts_path, fs::Permissions::from_mode(0o644));
    let hosts = "# IPv4.\n127.0.0.1   localhost.localdomain localhost\n\n# IPv6.\n::1         localhost.localdomain localhost ip6-localhost ip6-loopback\nfe00::0     ip6-localnet\nff00::0     ip6-mcastprefix\nff02::1     ip6-allnodes\nff02::2     ip6-allrouters\nff02::3     ip6-allhosts\n";
    fs::write(&hosts_path, hosts).map_err(|e| format!("write hosts: {}", e))?;
    msg_status(&format!("Creating file '{}'...", hosts_path));

    // /etc/environment
    let env_path = format!("{}/etc/environment", rootfs);
    let _ = fs::set_permissions(&env_path, fs::Permissions::from_mode(0o644));
    let mut env_content = String::new();
    for var in &[
        "ANDROID_ART_ROOT",
        "ANDROID_DATA",
        "ANDROID_I18N_ROOT",
        "ANDROID_ROOT",
        "ANDROID_RUNTIME_ROOT",
        "ANDROID_TZDATA_ROOT",
        "BOOTCLASSPATH",
        "DEX2OATBOOTCLASSPATH",
        "EXTERNAL_STORAGE",
    ] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                env_content.push_str(&format!("{}={}\n", var, val));
            }
        }
    }
    let term = std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string());
    env_content.push_str(&format!(
        "LANG=en_US.UTF-8\nMOZ_FAKE_NO_SANDBOX=1\nPATH={}\nPULSE_SERVER=127.0.0.1\nTERM={}\nTMPDIR=/tmp\n",
        default_path_env, term
    ));
    fs::write(&env_path, &env_content).map_err(|e| format!("write environment: {}", e))?;
    msg_status(&format!("Writing file '{}'...", env_path));

    // Fix PATH in common shell config files
    for f in &["/etc/bash.bashrc", "/etc/profile", "/etc/login.defs"] {
        let fp = format!("{}/{}", rootfs, f);
        if !Path::new(&fp).exists() {
            continue;
        }
        msg_status(&format!("Updating PATH in '{}' if needed...", fp));
        let _ = run_busybox_cmd(
            "sed",
            &[
                "-i",
                "-E",
                &format!(
                    "s@<(PATH=)(\"?[^\"[:space:]]+(\"|>|$))@{}\"{}\"@g",
                    "\\1", default_path_env
                ),
                &fp,
            ],
        );
    }

    // /etc/passwd, /etc/group, /etc/shadow, /etc/gshadow - register Android UIDs
    msg_status("Registering Android-specific UIDs and GIDs...");

    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    let username = run_cmd("id", &["-un"]).unwrap_or_else(|_| "root".to_string());

    for f in &["passwd", "shadow", "group", "gshadow"] {
        let fp = format!("{}/etc/{}", rootfs, f);
        let _ = fs::set_permissions(&fp, fs::Permissions::from_mode(0o644));
    }

    // passwd entry
    let passwd_path = format!("{}/etc/passwd", rootfs);
    let passwd_entry = format!(
        "aid_{}:x:{}:{}:proot-distro:/:/sbin/nologin\n",
        username, uid, gid
    );
    let mut passwd = fs::read_to_string(&passwd_path).unwrap_or_default();
    passwd.push_str(&passwd_entry);
    fs::write(&passwd_path, &passwd).map_err(|e| format!("write passwd: {}", e))?;

    // shadow entry
    let shadow_path = format!("{}/etc/shadow", rootfs);
    let shadow_entry = format!("aid_{}:*:18446:0:99999:7:::\n", username);
    let mut shadow = fs::read_to_string(&shadow_path).unwrap_or_default();
    shadow.push_str(&shadow_entry);
    fs::write(&shadow_path, &shadow).map_err(|e| format!("write shadow: {}", e))?;

    // group entries
    let group_names_str = run_cmd("id", &["-Gn"]).unwrap_or_else(|_| "root".to_string());
    let group_ids_str = run_cmd("id", &["-G"]).unwrap_or_else(|_| "0".to_string());

    let group_names: Vec<&str> = group_names_str.split_whitespace().collect();
    let group_ids: Vec<&str> = group_ids_str.split_whitespace().collect();

    let group_path = format!("{}/etc/group", rootfs);
    let mut group = fs::read_to_string(&group_path).unwrap_or_default();
    for (i, gname) in group_names.iter().enumerate() {
        let gid_val = group_ids.get(i).unwrap_or(&"0");
        group.push_str(&format!(
            "aid_{}:x:{}:root,aid_{}\n",
            gname, gid_val, username
        ));
    }
    fs::write(&group_path, &group).map_err(|e| format!("write group: {}", e))?;

    // gshadow entries
    let gshadow_path = format!("{}/etc/gshadow", rootfs);
    if Path::new(&gshadow_path).exists() {
        let mut gshadow = fs::read_to_string(&gshadow_path).unwrap_or_default();
        for gname in &group_names {
            gshadow.push_str(&format!("aid_{}:*::root,aid_{}\n", gname, username));
        }
        fs::write(&gshadow_path, &gshadow).map_err(|e| format!("write gshadow: {}", e))?;
    }

    // Fake /proc and /sys
    msg_status(&format!(
        "Creating fake /proc and /sys data in '{}'...",
        rootfs
    ));
    setup_fake_sysdata(rootfs)?;

    // distro_setup via proot if plugin has one
    let plugins_dir = get_plugins_dir();
    let plugin_path = format!("{}/{}.sh", plugins_dir, distro_name);
    if let Ok(content) = fs::read_to_string(&plugin_path) {
        if content.contains("distro_setup()") {
            msg_status("Running distribution-specific configuration steps...");

            let proot = get_native_proot();
            let rootfs_dir = rootfs.to_string();
            let setup_script = format!(
                "(. /etc/proot-distro/{}.sh 2>/dev/null; cd / && type distro_setup >/dev/null 2>&1 && distro_setup) >/dev/null 2>&1 || true",
                distro_name
            );

            let cache_dir = std::env::var("PROOT_TMP_DIR")
                .or_else(|_| std::env::var("TMPDIR"))
                .unwrap_or_else(|_| format!("{}/tmp", get_prefix()));

            let _ = Command::new(&proot)
                .env("PROOT_NO_SECCOMP", "1")
                .env("PROOT_L2S_DIR", format!("{}/.l2s", rootfs))
                .env("PROOT_TMP_DIR", &cache_dir)
                .env("TMPDIR", &cache_dir)
                .env("PROOT_LOADER", get_native_loader())
                .args([
                    "--link2symlink",
                    "-b",
                    &format!("{}:/etc/proot-distro", plugins_dir),
                    "-r",
                    &rootfs_dir,
                    "/bin/sh",
                    "-c",
                    &setup_script,
                ])
                .status();
        }
    }

    Ok(())
}

fn cleanup_on_failure(rootfs: &str, distro_name: &str) {
    let _ = fs::set_permissions(rootfs, fs::Permissions::from_mode(0o755));
    fn chmod_recursive(path: &Path) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o755));
                    chmod_recursive(&p);
                } else {
                    let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o644));
                }
            }
        }
    }
    chmod_recursive(Path::new(rootfs));
    let _ = fs::remove_dir_all(rootfs);

    let override_path = format!("{}/{}.override.sh", get_plugins_dir(), distro_name);
    if Path::new(&override_path).exists() {
        let _ = fs::remove_file(&override_path);
    }
}

pub fn command_install(
    distro_name: &str,
    override_alias: Option<&str>,
    override_tarball_url: Option<&str>,
    override_tarball_sha256: Option<&str>,
) -> Result<(), String> {
    let plugins_dir = get_plugins_dir();
    let installed_rootfs_dir = get_installed_rootfs_dir();
    let download_cache_dir = get_download_cache_dir();

    // Validate alias format if override-alias given
    let distro_name = if let Some(alias) = override_alias {
        if alias.is_empty() {
            return Err("argument to --override-alias should not be empty".to_string());
        }
        if alias.ends_with(".sh") {
            return Err("argument to --override-alias should not end with '.sh'".to_string());
        }
        if !alias
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        {
            return Err(
                "argument to --override-alias should start with an alphanumeric character"
                    .to_string(),
            );
        }
        if !alias
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '+' || c == '-')
        {
            return Err(
                "argument to --override-alias should consist of alphanumeric characters including symbols '_.+-'"
                    .to_string(),
            );
        }

        let override_path = format!("{}/{}.sh", plugins_dir, alias);
        let override_alt = format!("{}/{}.override.sh", plugins_dir, alias);
        if Path::new(&override_path).exists() || Path::new(&override_alt).exists() {
            return Err(format!(
                "distribution with alias '{}' already exists",
                alias
            ));
        }

        // Create .override.sh by copying original plugin
        let src_path = format!("{}/{}.sh", plugins_dir, distro_name);
        if !Path::new(&src_path).exists() {
            return Err(format!("unknown distribution '{}'", distro_name));
        }

        let plugins = load_plugins(Path::new(&plugins_dir));
        let orig_plugin = plugins
            .iter()
            .find(|p| p.alias == distro_name)
            .ok_or_else(|| format!("unknown distribution '{}'", distro_name))?;

        msg_status(&format!("Creating file '{}.override.sh'...", alias));

        let content = fs::read_to_string(&src_path).map_err(|e| format!("read plugin: {}", e))?;
        let new_content = content.replace(
            &format!("DISTRO_NAME=\"{}\"", orig_plugin.name),
            &format!("DISTRO_NAME=\"{} - {}\"", orig_plugin.name, alias),
        );
        fs::write(&override_alt, &new_content)
            .map_err(|e| format!("write override plugin: {}", e))?;

        alias.to_string()
    } else {
        distro_name.to_string()
    };

    // Check distro exists
    let plugin_path = format!("{}/{}.sh", plugins_dir, distro_name);
    let plugin_alt = format!("{}/{}.override.sh", plugins_dir, distro_name);
    if !Path::new(&plugin_path).exists() && !Path::new(&plugin_alt).exists() {
        println!();
        msg_error(&format!(
            "unknown distribution '{}' was requested to be installed.",
            distro_name
        ));
        println!();
        println!(
            "{}View supported distributions by: {}pr-cli list{}",
            CYAN, GREEN, RESET
        );
        println!();
        return Err(format!("unknown distribution '{}'", distro_name));
    };

    // Check not already installed
    let rootfs = format!("{}/{}", installed_rootfs_dir, distro_name);
    if Path::new(&rootfs).is_dir() {
        println!();
        msg_error(&format!(
            "distribution '{}' is already installed.",
            distro_name
        ));
        println!();
        println!(
            "{}Log in:     {}pr-cli login {}{}",
            CYAN, GREEN, distro_name, RESET
        );
        println!(
            "{}Reinstall:  {}pr-cli reset {}{}",
            CYAN, GREEN, distro_name, RESET
        );
        println!(
            "{}Uninstall:  {}pr-cli remove {}{}",
            CYAN, GREEN, distro_name, RESET
        );
        println!();
        return Err("already installed".to_string());
    }

    // Parse plugin
    let plugins = load_plugins(Path::new(&plugins_dir));
    let plugin = plugins
        .iter()
        .find(|p| p.alias == distro_name)
        .ok_or_else(|| format!("failed to parse plugin for '{}'", distro_name))?;

    // Detect device architecture
    let device_arch = detect_device_arch();

    msg_status(&format!("Installing {}...", plugin.name));

    // Create rootfs directory
    fs::create_dir_all(&rootfs).map_err(|e| format!("create rootfs dir: {}", e))?;
    msg_status(&format!("Creating directory '{}'...", rootfs));

    // Create .l2s directory
    let l2s_dir = format!("{}/.l2s", rootfs);
    fs::create_dir_all(&l2s_dir).map_err(|e| format!("create .l2s dir: {}", e))?;

    // Determine tarball URL and SHA256
    let arch = device_arch.as_str();
    let tarball = plugin.tarballs.get(arch).ok_or_else(|| {
        format!(
            "distribution download URL is not defined for CPU architecture '{}'",
            arch
        )
    })?;

    let tarball_url = override_tarball_url.unwrap_or(&tarball.url).to_string();
    let tarball_sha256 = override_tarball_sha256
        .unwrap_or(&tarball.sha256)
        .to_string();

    if tarball_url.is_empty() {
        msg_error(&format!(
            "distribution download URL is not defined for CPU architecture '{}'",
            arch
        ));
        cleanup_on_failure(&rootfs, &distro_name);
        return Err("no tarball URL".to_string());
    }

    // Download
    let archive_name = tarball_url
        .rsplit('/')
        .next()
        .unwrap_or("rootfs.tar.xz")
        .to_string();
    fs::create_dir_all(&download_cache_dir).map_err(|e| format!("create cache dir: {}", e))?;
    let archive_path = format!("{}/{}", download_cache_dir, archive_name);

    if Path::new(&archive_path).exists() {
        msg_status("Using cached rootfs archive...");
    } else {
        msg_status("Downloading rootfs archive...");
        msg_status(&format!("URL: {}", tarball_url));
        println!();
        if let Err(e) = download_file(&tarball_url, &archive_path, 3) {
            println!();
            msg_error("Download failure, please check your network connection.");
            cleanup_on_failure(&rootfs, &distro_name);
            return Err(format!("download failed: {}", e));
        }
        println!();
    }

    // SHA256 verification
    if !tarball_sha256.is_empty() {
        msg_status("Checking integrity, please wait...");
        if let Err(e) = verify_sha256(&tarball_sha256, &archive_path) {
            msg_error("Integrity checking failed. Try to redo installation again.");
            let _ = fs::remove_file(&archive_path);
            cleanup_on_failure(&rootfs, &distro_name);
            return Err(format!("sha256 verification failed: {}", e));
        }
    } else if override_tarball_url.is_some() {
        msg_error("Integrity checking of downloaded rootfs has been disabled.");
    }

    // Extract
    msg_status("Extracting rootfs, please wait...");

    extract_tarball(&archive_path, &rootfs, 1, &["dev"])?;

    // Validate rootfs structure
    if !Path::new(&format!("{}/etc", rootfs)).exists() {
        println!();
        msg_error(&format!(
            "rootfs of distribution '{}' has unexpected structure (no /etc directory).",
            distro_name
        ));
        cleanup_on_failure(&rootfs, &distro_name);
        return Err("no /etc in rootfs".to_string());
    }

    // Write config files
    write_config_files(&rootfs, &distro_name)?;

    msg_status("Finished.");
    println!();
    println!(
        "{}Log in with: {}pr-cli login {}{}",
        CYAN, GREEN, distro_name, RESET
    );
    println!();

    Ok(())
}
