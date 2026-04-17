use super::TestResult;
use std::fs;

pub fn probe() -> bool {
    true
}

pub fn test_file_io() -> TestResult {
    let dir = "/tmp/pit-fileio";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).map_err(|e| format!("mkdir: {}", e))?;

    let file = format!("{}/test.txt", dir);
    fs::write(&file, "hello").map_err(|e| format!("write: {}", e))?;

    let content = fs::read_to_string(&file).map_err(|e| format!("read: {}", e))?;
    if content != "hello" {
        return Err(format!("content mismatch: {:?}", content));
    }

    let renamed = format!("{}/renamed.txt", dir);
    fs::rename(&file, &renamed).map_err(|e| format!("rename: {}", e))?;

    fs::remove_dir_all(dir).map_err(|e| format!("rm: {}", e))?;
    Ok(())
}

pub fn test_symlink_ops() -> TestResult {
    let link = "/tmp/pit-symlink";
    let _ = fs::remove_file(link);
    std::os::unix::fs::symlink("/bin/sh", link).map_err(|e| format!("symlink: {}", e))?;
    let target = fs::read_link(link).map_err(|e| format!("readlink: {}", e))?;
    fs::remove_file(link).map_err(|e| format!("remove: {}", e))?;
    if target.to_str() == Some("/bin/sh") {
        Ok(())
    } else {
        Err(format!("expected /bin/sh, got {:?}", target))
    }
}

pub fn test_pipe() -> TestResult {
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "echo foo | cat | wc -c"])
        .output()
        .map_err(|e| format!("pipe: {}", e))?;
    let stdout = std::str::from_utf8(&out.stdout).unwrap_or("").trim();
    let count: usize = stdout.parse().unwrap_or(0);
    if count == 4 {
        Ok(())
    } else {
        Err(format!("expected 4 bytes, got {}", count))
    }
}

pub fn test_signal() -> TestResult {
    let out = std::process::Command::new("/bin/sh")
        .args([
            "-c",
            "trap 'echo caught' INT; kill -INT $$; echo not-caught",
        ])
        .output()
        .map_err(|e| format!("signal: {}", e))?;
    let stdout = std::str::from_utf8(&out.stdout).unwrap_or("");
    if stdout.contains("caught") || stdout.contains("not-caught") {
        Ok(())
    } else {
        Err(format!("unexpected signal output: {:?}", stdout))
    }
}

pub fn test_env() -> TestResult {
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "echo $HOME"])
        .env("HOME", "/root")
        .output()
        .map_err(|e| format!("env: {}", e))?;
    let stdout = std::str::from_utf8(&out.stdout).unwrap_or("").trim();
    if stdout == "/root" {
        Ok(())
    } else {
        Err(format!("expected /root, got {:?}", stdout))
    }
}
