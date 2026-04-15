use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

use crate::plugin::load_plugins;
use crate::shared::{
    get_bin_dir, get_download_cache_dir, get_installed_rootfs_dir, get_native_busybox,
    get_plugins_dir, msg_error, msg_status,
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

fn chmod_readable_recursive(path: &Path) {
    fn inner(p: &Path) {
        if let Ok(entries) = fs::read_dir(p) {
            for entry in entries.flatten() {
                let ep = entry.path();
                if ep.is_dir() {
                    let _ = fs::set_permissions(&ep, fs::Permissions::from_mode(0o755));
                    inner(&ep);
                } else {
                    let _ = fs::set_permissions(&ep, fs::Permissions::from_mode(0o644));
                }
            }
        }
    }
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o755));
    inner(path);
}

pub fn command_backup(distro_name: &str, output_path: Option<&str>) -> Result<(), String> {
    let installed_rootfs_dir = get_installed_rootfs_dir();
    let plugins_dir = get_plugins_dir();

    let plugins = load_plugins(Path::new(&plugins_dir));
    if !plugins.iter().any(|p| p.alias == distro_name) {
        println!();
        msg_error(&format!(
            "unknown distribution '{}' was requested for backup.",
            distro_name
        ));
        return Err("unknown distribution".to_string());
    }

    let rootfs = format!("{}/{}", installed_rootfs_dir, distro_name);
    if !Path::new(&rootfs).is_dir() {
        println!();
        msg_error(&format!("distribution '{}' is not installed.", distro_name));
        return Err("not installed".to_string());
    }

    let plugin = plugins.iter().find(|p| p.alias == distro_name);
    let display_name = plugin.map(|p| p.name.as_str()).unwrap_or(distro_name);

    let output = match output_path {
        Some(p) => p.to_string(),
        None => {
            msg_error("tarball output path is not specified. Use --output <path>.");
            return Err("no output path".to_string());
        }
    };

    if Path::new(&output).is_dir() {
        msg_error(&format!(
            "cannot write to '{}' because this path is a directory.",
            output
        ));
        return Err("output is directory".to_string());
    }
    if Path::new(&output).exists() {
        msg_error(&format!(
            "file '{}' already exists. Please specify a different name.",
            output
        ));
        return Err("output exists".to_string());
    }

    msg_status(&format!("Backing up {}...", display_name));
    msg_status(&format!("Tarball will be written to '{}'.", output));

    msg_status("Fixing file permissions in rootfs...");
    chmod_readable_recursive(Path::new(&rootfs));

    let plugin_file = if Path::new(&format!("{}/{}.sh", plugins_dir, distro_name)).exists() {
        format!("{}.sh", distro_name)
    } else {
        format!("{}.override.sh", distro_name)
    };

    let busybox = get_native_busybox();

    msg_status("Archiving the rootfs and plug-in...");
    let parent_rootfs = Path::new(&installed_rootfs_dir)
        .parent()
        .unwrap_or(Path::new("/"))
        .to_string_lossy()
        .to_string();
    let rootfs_basename = Path::new(&installed_rootfs_dir)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let parent_plugins = Path::new(&plugins_dir)
        .parent()
        .unwrap_or(Path::new("/"))
        .to_string_lossy()
        .to_string();
    let plugins_basename = Path::new(&plugins_dir)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let status = Command::new(&busybox)
        .arg0("busybox")
        .arg("tar")
        .args([
            "-c",
            "--auto-compress",
            "--warning=no-file-ignored",
            "-f",
            &output,
            "-C",
            &parent_plugins,
            &format!("{}/{}", plugins_basename, plugin_file),
            "-C",
            &parent_rootfs,
            &format!("{}/{}", rootfs_basename, distro_name),
        ])
        .status()
        .map_err(|e| format!("exec tar: {}", e))?;

    if !status.success() {
        let _ = fs::remove_file(&output);
        msg_error("Backup failed.");
        return Err("tar failed".to_string());
    }

    msg_status("Finished.");
    Ok(())
}

pub fn command_restore(tarball_path: &str) -> Result<(), String> {
    let installed_rootfs_dir = get_installed_rootfs_dir();
    let plugins_dir = get_plugins_dir();

    if !Path::new(tarball_path).exists() {
        msg_error(&format!("file '{}' does not exist.", tarball_path));
        return Err("file not found".to_string());
    }
    if Path::new(tarball_path).is_dir() {
        msg_error(&format!("path '{}' is a directory.", tarball_path));
        return Err("path is directory".to_string());
    }

    fs::create_dir_all(&installed_rootfs_dir)
        .map_err(|e| format!("create installed-rootfs dir: {}", e))?;
    fs::create_dir_all(&plugins_dir).map_err(|e| format!("create plugins dir: {}", e))?;

    msg_status("Extracting distribution plug-in and rootfs from the tarball...");

    let busybox = get_native_busybox();
    let parent_rootfs = Path::new(&installed_rootfs_dir)
        .parent()
        .unwrap_or(Path::new("/"))
        .to_string_lossy()
        .to_string();
    let rootfs_basename = Path::new(&installed_rootfs_dir)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let parent_plugins = Path::new(&plugins_dir)
        .parent()
        .unwrap_or(Path::new("/"))
        .to_string_lossy()
        .to_string();
    let plugins_basename = Path::new(&plugins_dir)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let status = Command::new(&busybox)
        .arg0("busybox")
        .arg("tar")
        .args([
            "-x",
            "--auto-compress",
            "--recursive-unlink",
            "--preserve-permissions",
            "-f",
            tarball_path,
            "-C",
            &parent_plugins,
            &format!("{}/", plugins_basename),
            "-C",
            &parent_rootfs,
            &format!("{}/", rootfs_basename),
        ])
        .status()
        .map_err(|e| format!("exec tar: {}", e))?;

    if !status.success() {
        msg_error("Restore failed.");
        return Err("tar extract failed".to_string());
    }

    msg_status("Finished.");
    Ok(())
}

pub fn command_rename(old_alias: &str, new_alias: &str) -> Result<(), String> {
    let installed_rootfs_dir = get_installed_rootfs_dir();
    let plugins_dir = get_plugins_dir();

    if old_alias == new_alias {
        msg_error("the original and new distribution aliases should not be same.");
        return Err("same alias".to_string());
    }

    if new_alias.is_empty() {
        msg_error("the new alias of distribution should not be empty.");
        return Err("empty alias".to_string());
    }
    if !new_alias
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        msg_error(
            "the new alias should start with an alphanumeric character and consist of alphanumeric characters including symbols '_.+-'.",
        );
        return Err("invalid alias".to_string());
    }
    if new_alias.ends_with(".sh") {
        msg_error("the new alias should not end with '.sh'.");
        return Err("alias ends with .sh".to_string());
    }

    let plugins = load_plugins(Path::new(&plugins_dir));
    if !plugins.iter().any(|p| p.alias == old_alias) {
        println!();
        msg_error(&format!(
            "unknown distribution '{}' was requested to be renamed.",
            old_alias
        ));
        println!();
        return Err("unknown distribution".to_string());
    }

    let old_rootfs = format!("{}/{}", installed_rootfs_dir, old_alias);
    if !Path::new(&old_rootfs).is_dir() {
        println!();
        msg_error(&format!(
            "cannot rename because the distribution '{}' is not installed.",
            old_alias
        ));
        return Err("not installed".to_string());
    }

    let new_rootfs = format!("{}/{}", installed_rootfs_dir, new_alias);
    if Path::new(&new_rootfs).is_dir() {
        msg_error(&format!(
            "cannot rename because rootfs directory for '{}' already exists.",
            new_alias
        ));
        return Err("target exists".to_string());
    }

    let new_plugin = format!("{}/{}.sh", plugins_dir, new_alias);
    let new_override = format!("{}/{}.override.sh", plugins_dir, new_alias);
    if Path::new(&new_plugin).exists() || Path::new(&new_override).exists() {
        msg_error(&format!(
            "distribution with alias '{}' already exists.",
            new_alias
        ));
        return Err("alias exists".to_string());
    }

    msg_status(&format!("Renaming '{}' to '{}'...", old_alias, new_alias));

    fs::rename(&old_rootfs, &new_rootfs).map_err(|e| format!("rename rootfs: {}", e))?;

    let old_plugin = format!("{}/{}.sh", plugins_dir, old_alias);
    let old_override = format!("{}/{}.override.sh", plugins_dir, old_alias);
    if Path::new(&old_override).exists() {
        fs::rename(&old_override, &new_override)
            .map_err(|e| format!("rename override plugin: {}", e))?;
    } else if Path::new(&old_plugin).exists() {
        let content = fs::read_to_string(&old_plugin).map_err(|e| format!("read plugin: {}", e))?;
        let plugin = plugins.iter().find(|p| p.alias == old_alias);
        if let Some(p) = plugin {
            let new_content = content.replace(
                &format!("DISTRO_NAME=\"{}\"", p.name),
                &format!("DISTRO_NAME=\"{} - {}\"", p.name, new_alias),
            );
            fs::write(&new_override, new_content)
                .map_err(|e| format!("write override plugin: {}", e))?;
        } else {
            fs::copy(&old_plugin, &new_override).map_err(|e| format!("copy plugin: {}", e))?;
        }
    }

    msg_status("Finished.");
    Ok(())
}

pub fn command_copy(src: &str, dst: &str) -> Result<(), String> {
    let installed_rootfs_dir = get_installed_rootfs_dir();

    let (src_dist, src_path) = parse_dist_path(src);
    let (dst_dist, dst_path) = parse_dist_path(dst);

    let src_full = resolve_path(&installed_rootfs_dir, src_dist.as_deref(), &src_path)?;
    let dst_full = resolve_path(&installed_rootfs_dir, dst_dist.as_deref(), &dst_path)?;

    if !Path::new(&src_full).exists() {
        msg_error(&format!(
            "can't copy '{}' because file does not exist.",
            src
        ));
        return Err("source not found".to_string());
    }

    msg_status(&format!("Source: '{}'", src_full));
    msg_status(&format!("Destination: '{}'", dst_full));

    if let Some(parent) = Path::new(&dst_full).parent() {
        if !parent.exists() {
            msg_status(&format!("Creating directory '{}'...", parent.display()));
            fs::create_dir_all(parent).map_err(|e| format!("create dir: {}", e))?;
        }
    }

    msg_status("Copying files, this may take a while...");

    let busybox = get_native_busybox();
    let status = Command::new(&busybox)
        .arg0("busybox")
        .args(["cp", "-a", &src_full, &dst_full])
        .status()
        .map_err(|e| format!("exec cp: {}", e))?;

    if !status.success() {
        msg_error("Copy failed.");
        return Err("cp failed".to_string());
    }

    msg_status("Finished.");
    Ok(())
}

fn parse_dist_path(input: &str) -> (Option<String>, String) {
    if let Some(pos) = input.find(':') {
        let dist = &input[..pos];
        let path = &input[pos + 1..];
        if !dist.is_empty() && !path.is_empty() {
            return (Some(dist.to_string()), path.to_string());
        }
    }
    (None, input.to_string())
}

fn resolve_path(
    installed_rootfs_dir: &str,
    dist: Option<&str>,
    path: &str,
) -> Result<String, String> {
    if let Some(d) = dist {
        let rootfs = format!("{}/{}", installed_rootfs_dir, d);
        if !Path::new(&rootfs).is_dir() {
            msg_error(&format!("distribution '{}' is not installed.", d));
            return Err("distro not installed".to_string());
        }
        let full = format!("{}/{}", rootfs, path);
        let canonical = Path::new(&full)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(&full).to_path_buf());
        Ok(canonical.to_string_lossy().to_string())
    } else {
        let canonical = Path::new(path)
            .canonicalize()
            .unwrap_or_else(|_| Path::new(path).to_path_buf());
        Ok(canonical.to_string_lossy().to_string())
    }
}
