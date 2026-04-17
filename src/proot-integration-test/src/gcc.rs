use super::TestResult;

pub fn probe() -> bool {
    std::process::Command::new("/bin/sh")
        .args(["-c", "cc --version >/dev/null 2>&1"])
        .status()
        .map(|o| o.success())
        .unwrap_or(false)
}

fn run_sh(cmd: &str) -> Result<std::process::Output, String> {
    std::process::Command::new("/bin/sh")
        .args(["-c", cmd])
        .output()
        .map_err(|e| format!("sh: {}", e))
}

pub fn test_search_dirs() -> TestResult {
    let out = run_sh("cc -print-search-dirs 2>&1")?;
    let stdout = std::str::from_utf8(&out.stdout).unwrap_or("");
    if stdout.contains("install:") && !stdout.contains(".l2s") {
        Ok(())
    } else {
        Err(format!("cc -print-search-dirs: {:?}", stdout))
    }
}

pub fn test_compile_c() -> TestResult {
    let src = "#include <stdio.h>\nint main(){printf(\"ok\\n\");return 0;}";
    std::fs::write("/tmp/pit-test.c", src).map_err(|e| format!("write: {}", e))?;
    let out = run_sh("cc /tmp/pit-test.c -o /tmp/pit-test 2>&1")?;
    if !out.status.success() {
        let _ = std::fs::remove_file("/tmp/pit-test.c");
        return Err(format!(
            "cc failed: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ));
    }
    let out2 = run_sh("/tmp/pit-test 2>&1")?;
    let _ = std::fs::remove_file("/tmp/pit-test");
    let _ = std::fs::remove_file("/tmp/pit-test.c");
    let stdout = std::str::from_utf8(&out2.stdout).unwrap_or("").trim();
    if stdout == "ok" {
        Ok(())
    } else {
        Err(format!("expected 'ok', got {:?}", stdout))
    }
}

pub fn test_proc_exe() -> TestResult {
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    if exe.to_str().unwrap_or("").contains(".l2s") {
        Err(format!("exe contains .l2s: {:?}", exe))
    } else {
        Ok(())
    }
}
