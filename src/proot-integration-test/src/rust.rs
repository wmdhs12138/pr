use super::TestResult;

pub fn probe() -> bool {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn test_rustc_version() -> TestResult {
    let out = std::process::Command::new("rustc")
        .args(["-vV"])
        .output()
        .map_err(|e| format!("rustc -vV: {}", e))?;
    let stdout = std::str::from_utf8(&out.stdout).unwrap_or("");
    if stdout.contains("rustc") && out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "rustc -vV: exit={}, {:?}",
            out.status.code().unwrap_or(-1),
            stdout
        ))
    }
}

pub fn test_rustc_compile() -> TestResult {
    let src = "fn main(){println!(\"rs-ok\");}";
    std::fs::write("/tmp/pit-test.rs", src).map_err(|e| format!("write: {}", e))?;
    let out = std::process::Command::new("rustc")
        .args(["/tmp/pit-test.rs", "-o", "/tmp/pit-test-rs"])
        .output()
        .map_err(|e| format!("rustc: {}", e))?;
    if !out.status.success() {
        let _ = std::fs::remove_file("/tmp/pit-test.rs");
        return Err(format!(
            "rustc failed: {:?}",
            std::str::from_utf8(&out.stderr).unwrap_or("")
        ));
    }
    let out2 = std::process::Command::new("/tmp/pit-test-rs")
        .output()
        .map_err(|e| format!("run: {}", e))?;
    let _ = std::fs::remove_file("/tmp/pit-test.rs");
    let _ = std::fs::remove_file("/tmp/pit-test-rs");
    let stdout = std::str::from_utf8(&out2.stdout).unwrap_or("").trim();
    if stdout == "rs-ok" {
        Ok(())
    } else {
        Err(format!("expected 'rs-ok', got {:?}", stdout))
    }
}

pub fn test_cargo_hello() -> TestResult {
    let _ = std::fs::remove_dir_all("/tmp/pit-cargo");
    let out = std::process::Command::new("cargo")
        .args(["new", "--vcs", "none", "/tmp/pit-cargo"])
        .output()
        .map_err(|e| format!("cargo new: {}", e))?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all("/tmp/pit-cargo");
        return Err(format!(
            "cargo new: {:?}",
            std::str::from_utf8(&out.stderr).unwrap_or("")
        ));
    }
    let out2 = std::process::Command::new("cargo")
        .args(["build"])
        .current_dir("/tmp/pit-cargo")
        .output()
        .map_err(|e| format!("cargo build: {}", e))?;
    let build_ok = out2.status.success();
    let _ = std::fs::remove_dir_all("/tmp/pit-cargo");
    if build_ok {
        Ok(())
    } else {
        Err(format!(
            "cargo build: {:?}",
            std::str::from_utf8(&out2.stderr).unwrap_or("")
        ))
    }
}
