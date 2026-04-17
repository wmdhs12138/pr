use super::TestResult;

pub fn probe() -> bool {
    true
}

pub fn test_fork_exec() -> TestResult {
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "echo ok"])
        .output()
        .map_err(|e| format!("fork+exec: {}", e))?;
    if out.status.success() && std::str::from_utf8(&out.stdout).unwrap_or("").trim() == "ok" {
        Ok(())
    } else {
        Err(format!(
            "exit={}, stdout={:?}",
            out.status.code().unwrap_or(-1),
            out.stdout
        ))
    }
}

pub fn test_stdout_piped() -> TestResult {
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "echo piped"])
        .stdout(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("stdout piped: {}", e))?;
    let stdout = std::str::from_utf8(&out.stdout).unwrap_or("").trim();
    if stdout == "piped" {
        Ok(())
    } else {
        Err(format!("expected 'piped', got {:?}", stdout))
    }
}

pub fn test_nested_spawn() -> TestResult {
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "/bin/sh -c '/bin/sh -c \"echo nested\"'"])
        .output()
        .map_err(|e| format!("nested spawn: {}", e))?;
    let stdout = std::str::from_utf8(&out.stdout).unwrap_or("").trim();
    if stdout == "nested" {
        Ok(())
    } else {
        Err(format!("expected 'nested', got {:?}", stdout))
    }
}

pub fn test_thread() -> TestResult {
    let handle = std::thread::spawn(|| {
        std::process::Command::new("/bin/sh")
            .args(["-c", "echo thread"])
            .output()
    });
    let out = handle
        .join()
        .map_err(|_| "thread panicked".to_string())?
        .map_err(|e| format!("thread spawn: {}", e))?;
    let stdout = std::str::from_utf8(&out.stdout).unwrap_or("").trim();
    if stdout == "thread" {
        Ok(())
    } else {
        Err(format!("expected 'thread', got {:?}", stdout))
    }
}
