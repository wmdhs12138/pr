use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::plugin::load_plugins;
use crate::shared::{
    get_download_cache_dir, get_installed_rootfs_dir, get_plugins_dir, msg_error, msg_status,
};

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
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o755));
}

pub fn command_remove(distro_name: &str, is_reset: bool) -> Result<(), String> {
    let installed_rootfs_dir = get_installed_rootfs_dir();
    let plugins_dir = get_plugins_dir();

    let plugins = load_plugins(Path::new(&plugins_dir));
    if !plugins.iter().any(|p| p.alias == distro_name) {
        println!();
        msg_error(&format!(
            "unknown distribution '{}' was requested to be removed.",
            distro_name
        ));
        println!();
        return Err("unknown distribution".to_string());
    }

    let rootfs = format!("{}/{}", installed_rootfs_dir, distro_name);
    if !Path::new(&rootfs).is_dir() {
        println!();
        msg_error(&format!("distribution '{}' is not installed.", distro_name));
        println!();
        return Err("not installed".to_string());
    }

    if !is_reset {
        let override_path = format!("{}/{}.override.sh", plugins_dir, distro_name);
        if Path::new(&override_path).exists() {
            msg_status(&format!("Deleting file '{}'...", override_path));
            let _ = fs::remove_file(&override_path);
        }
    }

    let plugin = plugins.iter().find(|p| p.alias == distro_name);
    let display_name = plugin.map(|p| p.name.as_str()).unwrap_or(distro_name);
    msg_status(&format!("Wiping the rootfs of {}...", display_name));

    chmod_recursive(Path::new(&rootfs));
    fs::remove_dir_all(&rootfs).map_err(|e| format!("failed to remove rootfs: {}", e))?;

    msg_status("Finished.");
    Ok(())
}

pub fn command_reset(distro_name: &str) -> Result<(), String> {
    let installed_rootfs_dir = get_installed_rootfs_dir();
    let plugins_dir = get_plugins_dir();

    let plugins = load_plugins(Path::new(&plugins_dir));
    if !plugins.iter().any(|p| p.alias == distro_name) {
        println!();
        msg_error(&format!(
            "unknown distribution '{}' was requested to be reset.",
            distro_name
        ));
        println!();
        return Err("unknown distribution".to_string());
    }

    let rootfs = format!("{}/{}", installed_rootfs_dir, distro_name);
    if !Path::new(&rootfs).is_dir() {
        println!();
        msg_error(&format!("distribution '{}' is not installed.", distro_name));
        println!();
        return Err("not installed".to_string());
    }

    command_remove(distro_name, true)?;

    crate::install::command_install(distro_name, None, None, None)?;

    Ok(())
}

pub fn command_clear_cache() -> Result<(), String> {
    let cache_dir = get_download_cache_dir();

    if !Path::new(&cache_dir).is_dir() {
        msg_status("Download cache is empty.");
        msg_status("Finished.");
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(&cache_dir)
        .map_err(|e| format!("read cache dir: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    if entries.is_empty() {
        msg_status("Download cache is empty.");
        msg_status("Finished.");
        return Ok(());
    }

    let total_size: u64 = entries
        .iter()
        .filter_map(|e| e.metadata().ok().map(|m| m.len()))
        .sum();
    let size_str = if total_size >= 1024 * 1024 {
        format!("{:.1}MB", total_size as f64 / (1024.0 * 1024.0))
    } else if total_size >= 1024 {
        format!("{:.1}KB", total_size as f64 / 1024.0)
    } else {
        format!("{}B", total_size)
    };

    msg_status("Clearing cache files...");

    for entry in &entries {
        msg_status(&format!("Deleting '{}'", entry.path().display()));
        let _ = fs::remove_file(entry.path());
    }

    msg_status(&format!("Reclaimed {} of disk space.", size_str));
    msg_status("Finished.");
    Ok(())
}
