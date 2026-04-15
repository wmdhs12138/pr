use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::shared::*;

struct PasswdEntry {
    uid: u32,
    gid: u32,
    home: String,
    shell: String,
}

fn parse_passwd_entry(line: &str) -> Option<PasswdEntry> {
    let fields: Vec<&str> = line.split(':').collect();
    if fields.len() < 7 {
        return None;
    }
    Some(PasswdEntry {
        uid: fields[2].parse().ok()?,
        gid: fields[3].parse().ok()?,
        home: fields[5].to_string(),
        shell: fields[6].to_string(),
    })
}

fn find_user_in_passwd(passwd_path: &str, user: &str) -> Result<PasswdEntry, String> {
    let content = fs::read_to_string(passwd_path).map_err(|e| format!("read passwd: {}", e))?;
    for line in content.lines() {
        if let Some(entry) = parse_passwd_entry(line) {
            if line.starts_with(&format!("{}:", user)) {
                return Ok(entry);
            }
        }
    }
    Err(format!("no user '{}' defined in /etc/passwd", user))
}

fn can_read_dir(path: &str) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    let mode = meta.permissions().mode();
    (mode & 0o5) != 0
}

fn can_read_file(path: &str) -> bool {
    fs::read(path).is_ok()
}

fn can_list_dir(path: &str) -> bool {
    fs::read_dir(path).is_ok()
}

fn update_etc_environment(rootfs: &str) {
    let env_path = format!("{}/etc/environment", rootfs);
    let _ = fs::set_permissions(&env_path, fs::Permissions::from_mode(0o644));

    let mut lines: Vec<String> = Vec::new();
    if let Ok(content) = fs::read_to_string(&env_path) {
        for line in content.lines() {
            let key = line.split('=').next().unwrap_or("");
            if !key.is_empty()
                && !key.starts_with("ANDROID_")
                && key != "BOOTCLASSPATH"
                && key != "DEX2OATBOOTCLASSPATH"
            {
                lines.push(line.to_string());
            }
        }
    }

    for var in &[
        "ANDROID_ART_ROOT",
        "ANDROID_DATA",
        "ANDROID_I18N_ROOT",
        "ANDROID_ROOT",
        "ANDROID_RUNTIME_ROOT",
        "ANDROID_TZDATA_ROOT",
        "BOOTCLASSPATH",
        "DEX2OATBOOTCLASSPATH",
    ] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                lines.push(format!("{}={}", var, val));
            }
        }
    }

    let _ = fs::write(&env_path, lines.join("\n") + "\n");
}

pub fn command_login(
    distro_name: &str,
    user: &str,
    isolated: bool,
    no_link2symlink: bool,
    custom_bind: &[String],
    extra_args: &[String],
) -> Result<(), String> {
    let prefix = get_prefix();
    let bin_dir = get_bin_dir();
    let installed_rootfs_dir = get_installed_rootfs_dir();
    let rootfs = format!("{}/{}", installed_rootfs_dir, distro_name);

    if !Path::new(&rootfs).is_dir() {
        println!();
        msg_error(&format!("distribution '{}' is not installed.", distro_name));
        println!();
        return Err("not installed".to_string());
    }

    let passwd_path = format!("{}/etc/passwd", rootfs);
    if !Path::new(&passwd_path).exists() {
        msg_error("the selected distribution doesn't have /etc/passwd.");
        return Err("no /etc/passwd".to_string());
    }

    let entry = find_user_in_passwd(&passwd_path, user)?;

    update_etc_environment(&rootfs);

    let l2s_dir = format!("{}/.l2s", rootfs);
    if Path::new(&l2s_dir).is_dir() {
        std::env::set_var("PROOT_L2S_DIR", &l2s_dir);
    }

    // Build proot arguments
    let mut args: Vec<String> = Vec::new();

    // Custom binds
    for bnd in custom_bind {
        args.push(format!("--bind={}", bnd));
    }

    // Non-isolated: bind Android system dirs
    if !isolated {
        for data_dir in &[
            "/data/app",
            "/data/dalvik-cache",
            "/data/misc/apexdata/com.android.art/dalvik-cache",
        ] {
            if Path::new(data_dir).is_dir() && can_read_dir(data_dir) {
                args.push(format!("--bind={}", data_dir));
            }
        }

        let apps_dir = format!("/data/data/id.or.oo.pr/files/apps");
        if Path::new(&apps_dir).is_dir() {
            args.push(format!("--bind={}", apps_dir));
        }

        args.push("--bind=/data/data/id.or.oo.pr/cache".to_string());
        args.push("--bind=/data/data/id.or.oo.pr".to_string());

        // Storage binds
        if can_list_dir("/storage") {
            args.push("--bind=/storage".to_string());
            args.push("--bind=/storage/emulated/0:/sdcard".to_string());
        } else {
            let storage_path = if can_list_dir("/storage/self/primary/") {
                Some("/storage/self/primary")
            } else if can_list_dir("/storage/emulated/0/") {
                Some("/storage/emulated/0")
            } else if can_list_dir("/sdcard/") {
                Some("/sdcard")
            } else {
                None
            };

            if let Some(sp) = storage_path {
                args.push(format!("--bind={}:/sdcard", sp));
                args.push(format!("--bind={}:/storage/emulated/0", sp));
                args.push(format!("--bind={}:/storage/self/primary", sp));
            }
        }
    }

    // System mounts (non-isolated or with CPU emulator)
    if !isolated {
        for system_mnt in &[
            "/apex",
            "/odm",
            "/product",
            "/system",
            "/system_ext",
            "/vendor",
            "/linkerconfig/ld.config.txt",
            "/linkerconfig/com.android.art/ld.config.txt",
            "/plat_property_contexts",
            "/property_contexts",
        ] {
            let p = Path::new(system_mnt);
            if !p.exists() {
                continue;
            }
            let real = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
            let real_str = real.to_string_lossy();
            if real.is_dir() {
                if can_read_dir(&real_str) {
                    args.push(format!("--bind={}", real_str));
                }
            } else if real.is_file() {
                if can_read_file(&real_str) {
                    args.push(format!("--bind={}", real_str));
                }
            }
        }

        args.push(format!("--bind={}", prefix));
    }

    // /tmp -> /dev/shm
    let tmp_dir = format!("{}/tmp", rootfs);
    if !Path::new(&tmp_dir).is_dir() {
        let _ = fs::create_dir_all(&tmp_dir);
        let _ = fs::set_permissions(&tmp_dir, fs::Permissions::from_mode(0o1777));
    }
    args.push(format!("--bind={}/tmp:/dev/shm", rootfs));

    // Fake /proc entries
    for (fake, real) in &[
        ("proc/.loadavg", "/proc/loadavg"),
        ("proc/.stat", "/proc/stat"),
        ("proc/.uptime", "/proc/uptime"),
        ("proc/.version", "/proc/version"),
        ("proc/.vmstat", "/proc/vmstat"),
    ] {
        if !can_read_file(real) {
            args.push(format!("--bind={}/{}:{}", rootfs, fake, real));
        }
    }
    if Path::new("/sys/fs/selinux").exists() {
        args.push(format!("{}/sys/.empty:/sys/fs/selinux", rootfs));
    }

    // /proc/self/fd binds for stdin/stdout/stderr
    for (fd, name) in &[(0, "stdin"), (1, "stdout"), (2, "stderr")] {
        let fd_path = format!("/proc/self/fd/{}", fd);
        if fs::read_link(&fd_path).is_ok() {
            args.push(format!("--bind={}:{}", fd_path, name));
        }
    }

    // Core binds
    args.push("--bind=/proc/self/fd:/dev/fd".to_string());
    args.push("--bind=/dev/urandom:/dev/random".to_string());
    args.push("--bind=/sys".to_string());
    args.push("--bind=/proc".to_string());
    args.push("--bind=/dev".to_string());

    // -L (fix lstat)
    args.push("-L".to_string());

    // Fake kernel
    let hostname = "localhost";
    let kernel_release = std::env::var("PROOT_DISTRO_KERNEL_RELEASE")
        .unwrap_or_else(|_| DEFAULT_FAKE_KERNEL_RELEASE.to_string());
    let machine = std::env::var("PROOT_DISTRO_MACHINE").unwrap_or_else(|_| "aarch64".to_string());
    args.push(format!(
        "--kernel-release=\\Linux\\{}\\{}\\{}\\{}\\localdomain\\-1\\",
        hostname, kernel_release, DEFAULT_FAKE_KERNEL_VERSION, machine
    ));

    // link2symlink
    if !no_link2symlink {
        args.push("--link2symlink".to_string());
    }

    // sysvipc
    args.push("--sysvipc".to_string());

    // kill-on-exit
    args.push("--kill-on-exit".to_string());

    // Rootfs, cwd, change-id
    let login_wd = entry.home.clone();
    args.push(format!("--change-id={}:{}", entry.uid, entry.gid));
    args.push(format!("--rootfs={}", rootfs));
    args.push(format!("--cwd={}", login_wd));

    // Build environment for login shell
    let mut login_env: Vec<String> = vec![format!("PATH={}", get_default_path_env())];

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
                login_env.push(format!("{}={}", var, val));
            }
        }
    }

    // Read /etc/environment from rootfs
    let env_file = format!("{}/etc/environment", rootfs);
    if let Ok(content) = fs::read_to_string(&env_file) {
        for line in content.lines() {
            if line.contains('=') && !line.is_empty() {
                let clean = line.trim_matches('\'').trim_matches('"');
                if clean.contains('=')
                    && !login_env
                        .iter()
                        .any(|e| e.split('=').next() == clean.split('=').next())
                {
                    login_env.push(clean.to_string());
                }
            }
        }
    }

    // Build command part: /usr/bin/env -i ENV... SHELL -l [extra_args]
    let mut cmd_args: Vec<String> = Vec::new();
    cmd_args.push("/usr/bin/env".to_string());
    cmd_args.push("-i".to_string());
    cmd_args.extend(login_env);
    cmd_args.push(format!(
        "COLORTERM={}",
        std::env::var("COLORTERM").unwrap_or_default()
    ));
    cmd_args.push(format!("HOME={}", entry.home));
    cmd_args.push(format!("USER={}", user));
    cmd_args.push(format!(
        "TERM={}",
        std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string())
    ));
    cmd_args.push(entry.shell.clone());
    cmd_args.push("-l".to_string());

    if !extra_args.is_empty() {
        cmd_args.push("-c".to_string());
        cmd_args.push(extra_args.join(" "));
    }

    // The proot binary
    let proot = get_native_proot();

    let mut full_args = Vec::new();
    full_args.extend(args);
    full_args.extend(cmd_args);

    let err = Command::new(&proot)
        .env("PROOT_NO_SECCOMP", "1")
        .args(&full_args)
        .exec();

    Err(format!("exec proot failed: {}", err))
}
