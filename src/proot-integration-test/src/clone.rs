use super::TestResult;
use std::thread;

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

pub fn test_concurrent_spawn() -> TestResult {
    let n = 10;
    let handles: Vec<thread::JoinHandle<Result<String, String>>> = (0..n)
        .map(|i| {
            thread::spawn(move || {
                let out = std::process::Command::new("/bin/sh")
                    .args(["-c", &format!("echo {}", i)])
                    .stdout(std::process::Stdio::piped())
                    .output()
                    .map_err(|e| format!("spawn {}: {}", i, e))?;
                let stdout = std::str::from_utf8(&out.stdout)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if stdout == i.to_string() {
                    Ok(stdout)
                } else {
                    Err(format!("expected '{}' got {:?}", i, stdout))
                }
            })
        })
        .collect();

    let mut results = Vec::new();
    for (i, h) in handles.into_iter().enumerate() {
        match h.join() {
            Ok(Ok(s)) => results.push(s),
            Ok(Err(e)) => return Err(format!("spawn {}: {}", i, e)),
            Err(_) => return Err(format!("spawn {}: thread panicked", i)),
        }
    }

    if results.len() == n {
        Ok(())
    } else {
        Err(format!("expected {} results, got {}", n, results.len()))
    }
}
