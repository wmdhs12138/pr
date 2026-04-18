use super::TestResult;

fn run_sh(cmd: &str) -> Result<std::process::Output, String> {
    std::process::Command::new("/bin/sh")
        .args(["-c", cmd])
        .output()
        .map_err(|e| format!("sh: {}", e))
}

pub fn probe() -> bool {
    std::process::Command::new("/bin/sh")
        .args(["-c", "rustc --version >/dev/null 2>&1"])
        .status()
        .map(|o| o.success())
        .unwrap_or(false)
}

pub fn test_rustc_version() -> TestResult {
    let out = run_sh("rustc -vV 2>&1")?;
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
    let out = run_sh("rustc /tmp/pit-test.rs -o /tmp/pit-test-rs 2>&1")?;
    if !out.status.success() {
        let _ = std::fs::remove_file("/tmp/pit-test.rs");
        return Err(format!(
            "rustc failed: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ));
    }
    let out2 = run_sh("/tmp/pit-test-rs 2>&1")?;
    let _ = std::fs::remove_file("/tmp/pit-test.rs");
    let _ = std::fs::remove_file("/tmp/pit-test-rs");
    let stdout = std::str::from_utf8(&out2.stdout).unwrap_or("").trim();
    if stdout == "rs-ok" {
        Ok(())
    } else {
        Err(format!("expected 'rs-ok', got {:?}", stdout))
    }
}

pub fn test_cargo_no_vcs() -> TestResult {
    let _ = std::fs::remove_dir_all("/tmp/pit-cargo-novcs");
    let out = run_sh("cargo new --vcs none /tmp/pit-cargo-novcs 2>&1")?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all("/tmp/pit-cargo-novcs");
        return Err(format!(
            "cargo new --vcs none: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ));
    }
    let out2 = run_sh("cd /tmp/pit-cargo-novcs && cargo build 2>&1")?;
    let build_ok = out2.status.success();
    let _ = std::fs::remove_dir_all("/tmp/pit-cargo-novcs");
    if build_ok {
        Ok(())
    } else {
        Err(format!(
            "cargo build --vcs none: {:?}",
            std::str::from_utf8(&out2.stdout).unwrap_or("")
        ))
    }
}

pub fn test_cargo_with_vcs() -> TestResult {
    let _ = std::fs::remove_dir_all("/tmp/pit-cargo-vcs");

    if std::path::Path::new("/tmp/pit-cargo-vcs/.git/config.lock").exists() {
        return Err("stale .git/config.lock exists after remove_dir_all".to_string());
    }

    let cfg = run_sh("mkdir -p /root/.config/git && touch /root/.config/git/config 2>&1")?;
    if !cfg.status.success() {
        return Err(format!(
            "mkdir git config: {:?}",
            std::str::from_utf8(&cfg.stdout).unwrap_or("")
        ));
    }

    let out = run_sh("cargo new /tmp/pit-cargo-vcs 2>&1")?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all("/tmp/pit-cargo-vcs");
        return Err(format!(
            "cargo new: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ));
    }
    let out2 = run_sh("cd /tmp/pit-cargo-vcs && cargo build 2>&1")?;
    let build_ok = out2.status.success();
    let _ = std::fs::remove_dir_all("/tmp/pit-cargo-vcs");
    if build_ok {
        Ok(())
    } else {
        Err(format!(
            "cargo build: {:?}",
            std::str::from_utf8(&out2.stdout).unwrap_or("")
        ))
    }
}
