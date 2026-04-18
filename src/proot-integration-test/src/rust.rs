use super::TestResult;
use std::time::Instant;

const CMD_TIMEOUT_SECS: u64 = 30;

fn run_sh_timed(label: &str, cmd: &str) -> Result<std::process::Output, String> {
    eprintln!("  [rust] {} start: {}", label, cmd);
    let start = Instant::now();
    let child = std::process::Command::new("/bin/sh")
        .args(["-c", cmd])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("spawn: {}", e))?;

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            return Err(format!("wait: {} after {:.1}s", e, start.elapsed().as_secs_f64()));
        }
    };
    let elapsed = start.elapsed().as_secs_f64();
    let stdout = std::str::from_utf8(&output.stdout).unwrap_or("");
    let stderr = std::str::from_utf8(&output.stderr).unwrap_or("");
    eprintln!(
        "  [rust] {} done in {:.1}s exit={}",
        label,
        elapsed,
        output.status.code().unwrap_or(-1)
    );
    if elapsed > CMD_TIMEOUT_SECS as f64 {
        eprintln!("  [rust] WARNING: {} took {:.1}s (slow)", label, elapsed);
    }
    if !output.status.success() {
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        eprintln!("  [rust] {} failed: {:.200}", label, detail);
    }
    Ok(output)
}

pub fn probe() -> bool {
    std::path::Path::new("/usr/bin/rustc").exists() || std::path::Path::new("/usr/local/bin/rustc").exists()
}

pub fn test_rustc_version() -> TestResult {
    let out = run_sh_timed("rustc -vV", "rustc -vV 2>&1")?;
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

    let out = run_sh_timed("rustc compile", "rustc /tmp/pit-test.rs -o /tmp/pit-test-rs 2>&1")?;
    if !out.status.success() {
        let _ = std::fs::remove_file("/tmp/pit-test.rs");
        return Err(format!(
            "rustc failed: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ));
    }

    let out2 = run_sh_timed("run binary", "/tmp/pit-test-rs 2>&1")?;
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

    let out = run_sh_timed("cargo new", "cargo new --vcs none /tmp/pit-cargo-novcs 2>&1")?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all("/tmp/pit-cargo-novcs");
        return Err(format!(
            "cargo new --vcs none: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ));
    }

    let out2 = run_sh_timed("cargo build", "cd /tmp/pit-cargo-novcs && cargo build 2>&1")?;
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

    let cfg = run_sh_timed("mkdir git config", "mkdir -p /root/.config/git && touch /root/.config/git/config 2>&1")?;
    if !cfg.status.success() {
        return Err(format!(
            "mkdir git config: {:?}",
            std::str::from_utf8(&cfg.stdout).unwrap_or("")
        ));
    }

    let out = run_sh_timed("cargo new (vcs)", "cargo new /tmp/pit-cargo-vcs 2>&1")?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all("/tmp/pit-cargo-vcs");
        return Err(format!(
            "cargo new: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ));
    }

    let out2 = run_sh_timed("cargo build (vcs)", "cd /tmp/pit-cargo-vcs && cargo build 2>&1")?;
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
