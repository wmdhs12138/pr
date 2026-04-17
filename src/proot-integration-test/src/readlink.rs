use super::TestResult;
use std::fs;

pub fn probe() -> bool {
    std::path::Path::new("/bin/sh").exists()
}

pub fn test_symlink_resolve() -> TestResult {
    let tmp = "/tmp/pit-readlink-sym";
    let _ = fs::remove_file(tmp);
    std::os::unix::fs::symlink("/bin/sh", tmp).map_err(|e| format!("symlink: {}", e))?;
    let target = fs::read_link(tmp).map_err(|e| format!("readlink: {}", e))?;
    let _ = fs::remove_file(tmp);
    if target.to_str() == Some("/bin/sh") {
        Ok(())
    } else {
        Err(format!("expected /bin/sh, got {:?}", target))
    }
}

pub fn test_realpath_no_l2s() -> TestResult {
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "realpath /usr/bin/cc 2>&1 || realpath /bin/sh 2>&1"])
        .output()
        .map_err(|e| format!("realpath: {}", e))?;
    let path = std::str::from_utf8(&out.stdout).unwrap_or("").trim();
    if path.contains(".l2s") {
        Err(format!("realpath contains .l2s: {}", path))
    } else if path.is_empty() {
        Err("realpath returned empty".to_string())
    } else {
        Ok(())
    }
}

pub fn test_readlink_einval() -> TestResult {
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "readlink /usr/bin/gcc 2>&1; echo exit=$?"])
        .output()
        .map_err(|e| format!("readlink: {}", e))?;
    let output = std::str::from_utf8(&out.stdout).unwrap_or("");
    if output.contains("exit=0") && !output.contains(".l2s") {
        Ok(())
    } else if output.contains("Invalid") || output.contains("exit=1") {
        Ok(())
    } else {
        Err(format!("unexpected readlink output: {:?}", output))
    }
}

pub fn test_proc_self_exe() -> TestResult {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    let path = exe.to_str().unwrap_or("");
    if path.contains(".l2s") {
        Err(format!("/proc/self/exe contains .l2s: {}", path))
    } else {
        Ok(())
    }
}

pub fn test_lstat_stat() -> TestResult {
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "stat -c '%s' /bin/sh && stat -L -c '%s' /bin/sh"])
        .output()
        .map_err(|e| format!("stat: {}", e))?;
    let lines: Vec<&str> = std::str::from_utf8(&out.stdout)
        .unwrap_or("")
        .lines()
        .collect();
    if lines.len() >= 2 {
        Ok(())
    } else {
        Err(format!("expected 2 stat lines, got {:?}", lines))
    }
}
