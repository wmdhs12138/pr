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
