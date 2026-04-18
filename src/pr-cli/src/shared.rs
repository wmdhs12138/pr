pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const DEFAULT_FAKE_KERNEL_RELEASE: &str = "6.17.0-pr";
pub const DEFAULT_FAKE_KERNEL_VERSION: &str =
    "#1 SMP PREEMPT_DYNAMIC Fri, 10 Oct 2025 00:00:00 +0000";
pub const DEFAULT_PRIMARY_NAMESERVER: &str = "8.8.8.8";
pub const DEFAULT_SECONDARY_NAMESERVER: &str = "8.8.4.4";
pub const DEFAULT_PATH_ENV_SUFFIX: &str =
    "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/local/games:/usr/games";

pub fn get_prefix() -> String {
    std::env::var("APP_PREFIX").unwrap_or_else(|_| "/data/data/id.or.oo.pr/files/usr".to_string())
}

pub fn get_bin_dir() -> String {
    format!("{}/bin", get_prefix())
}

pub fn get_plugins_dir() -> String {
    format!("{}/etc/proot-distro", get_prefix())
}

pub fn get_installed_rootfs_dir() -> String {
    format!("{}/var/lib/proot-distro/installed-rootfs", get_prefix())
}

pub fn get_download_cache_dir() -> String {
    format!("{}/var/lib/proot-distro/dlcache", get_prefix())
}

pub fn get_default_path_env() -> String {
    format!("{}:{}", DEFAULT_PATH_ENV_SUFFIX, get_prefix())
}

pub fn get_native_lib_dir() -> String {
    std::fs::read_link("/proc/self/exe")
        .ok()
        .and_then(|p| p.parent().map(|p| p.display().to_string()))
        .unwrap_or_else(|| get_bin_dir())
}

pub fn get_native_busybox() -> String {
    format!("{}/libbusybox.so", get_native_lib_dir())
}

pub fn get_native_proot() -> String {
    format!("{}/libproot.so", get_native_lib_dir())
}

pub fn get_native_bash() -> String {
    format!("{}/libbash.so", get_native_lib_dir())
}

pub fn get_native_loader() -> String {
    format!("{}/libproot-loader.so", get_native_lib_dir())
}

pub fn msg_status(text: &str) {
    println!(
        "{}\x1b[1;34m[\x1b[32m*\x1b[1;34m\x1b[36m {}\x1b[0m",
        "", text
    );
}

pub fn msg_error(text: &str) {
    println!(
        "{}\x1b[1;34m[\x1b[31m!\x1b[1;34m\x1b[36m {}\x1b[0m",
        "", text
    );
}

pub fn can_read_dir(path: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let mode = meta.permissions().mode();
    (mode & 0o5) != 0
}

pub fn can_read_file(path: &str) -> bool {
    std::fs::read(path).is_ok()
}

pub fn can_list_dir(path: &str) -> bool {
    std::fs::read_dir(path).is_ok()
}

pub fn build_proot_args(
    rootfs: &str,
    isolated: bool,
    no_link2symlink: bool,
    custom_bind: &[String],
) -> Vec<String> {
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    let prefix = get_prefix();
    let cache_dir = std::env::var("PROOT_TMP_DIR")
        .or_else(|_| std::env::var("TMPDIR"))
        .unwrap_or_else(|_| format!("{}/tmp", prefix));

    let l2s_dir = format!("{}/.l2s", rootfs);
    if Path::new(&l2s_dir).is_dir() {
        std::env::set_var("PROOT_L2S_DIR", &l2s_dir);
    }

    let mut args: Vec<String> = Vec::new();

    for bnd in custom_bind {
        args.push(format!("--bind={}", bnd));
    }

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

        let apps_dir = "/data/data/id.or.oo.pr/files/apps";
        if Path::new(apps_dir).is_dir() {
            args.push(format!("--bind={}", apps_dir));
        }

        args.push("--bind=/data/data/id.or.oo.pr/cache".to_string());
        args.push("--bind=/data/data/id.or.oo.pr".to_string());

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

    let tmp_dir = format!("{}/tmp", rootfs);
    if !Path::new(&tmp_dir).is_dir() {
        let _ = std::fs::create_dir_all(&tmp_dir);
        let _ = std::fs::set_permissions(&tmp_dir, std::fs::Permissions::from_mode(0o1777));
    }
    args.push(format!("--bind={}/tmp:/dev/shm", rootfs));

    args.push("--bind=/dev".to_string());
    args.push("--bind=/proc".to_string());
    args.push("--bind=/sys".to_string());

    args.push("--bind=/proc/self/fd:/dev/fd".to_string());
    args.push("--bind=/dev/urandom:/dev/random".to_string());

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
        args.push(format!("--bind={}/sys/.empty:/sys/fs/selinux", rootfs));
    }

    args.push(format!("--bind={}:{}", cache_dir, "/tmp"));

    args.push("-L".to_string());

    let hostname = "localhost";
    let kernel_release = std::env::var("PROOT_DISTRO_KERNEL_RELEASE")
        .unwrap_or_else(|_| DEFAULT_FAKE_KERNEL_RELEASE.to_string());
    let machine = std::env::var("PROOT_DISTRO_MACHINE").unwrap_or_else(|_| "aarch64".to_string());
    args.push(format!(
        "--kernel-release=Linux\\{}\\{}\\{}\\{}\\localdomain\\-1\\",
        hostname, kernel_release, DEFAULT_FAKE_KERNEL_VERSION, machine
    ));

    if !no_link2symlink {
        args.push("--link2symlink".to_string());
    }

    args.push("--kill-on-exit".to_string());

    args.push(format!("--rootfs={}", rootfs));
    args.push("--cwd=/".to_string());

    args
}

pub fn build_proot_runtime_env() -> Vec<(&'static str, String)> {
    let cache_dir = std::env::var("PROOT_TMP_DIR")
        .or_else(|_| std::env::var("TMPDIR"))
        .unwrap_or_else(|_| format!("{}/tmp", get_prefix()));

    vec![
        ("PROOT_NO_SECCOMP", "1".to_string()),
        (
            "PROOT_L2S_DIR",
            std::env::var("PROOT_L2S_DIR").unwrap_or_default(),
        ),
        ("PROOT_TMP_DIR", cache_dir.clone()),
        ("TMPDIR", cache_dir),
        ("PROOT_LOADER", get_native_loader()),
    ]
}

pub fn build_proot_child_env() -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = vec![
        ("PATH".to_string(), get_default_path_env()),
        ("HOME".to_string(), "/root".to_string()),
        ("USER".to_string(), "root".to_string()),
        (
            "TERM".to_string(),
            std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string()),
        ),
        ("LANG".to_string(), "en_US.UTF-8".to_string()),
        ("TMPDIR".to_string(), "/tmp".to_string()),
    ];

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
                env.push((var.to_string(), val));
            }
        }
    }

    env
}
