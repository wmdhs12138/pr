use super::TestResult;

pub fn probe() -> bool {
    std::process::Command::new("cc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn test_search_dirs() -> TestResult {
    let out = std::process::Command::new("cc")
        .args(["-print-search-dirs"])
        .output()
        .map_err(|e| format!("cc: {}", e))?;
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
    let out = std::process::Command::new("cc")
        .args(["/tmp/pit-test.c", "-o", "/tmp/pit-test"])
        .output()
        .map_err(|e| format!("cc compile: {}", e))?;
    if !out.status.success() {
        return Err(format!(
            "cc failed: {:?}",
            std::str::from_utf8(&out.stderr).unwrap_or("")
        ));
    }
    let out2 = std::process::Command::new("/tmp/pit-test")
        .output()
        .map_err(|e| format!("run: {}", e))?;
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
