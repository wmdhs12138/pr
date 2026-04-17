use super::TestResult;

fn run_sh(cmd: &str) -> Result<std::process::Output, String> {
    std::process::Command::new("/bin/sh")
        .args(["-c", cmd])
        .output()
        .map_err(|e| format!("sh: {}", e))
}

pub fn probe() -> bool {
    std::process::Command::new("/bin/sh")
        .args(["-c", "git --version >/dev/null 2>&1"])
        .status()
        .map(|o| o.success())
        .unwrap_or(false)
}

pub fn test_git_init() -> TestResult {
    let _ = std::fs::remove_dir_all("/tmp/pit-git");
    std::fs::create_dir_all("/tmp/pit-git").map_err(|e| format!("mkdir: {}", e))?;
    let out = run_sh("cd /tmp/pit-git && git init 2>&1")?;
    let _ = std::fs::remove_dir_all("/tmp/pit-git");
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git init: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ))
    }
}

pub fn test_git_config() -> TestResult {
    let out = run_sh("git config --global user.name test 2>&1")?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git config: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ))
    }
}

pub fn test_cargo_new_git() -> TestResult {
    let _ = std::fs::remove_dir_all("/tmp/pit-cargo-git");
    let out = run_sh("cargo new /tmp/pit-cargo-git 2>&1")?;
    let _ = std::fs::remove_dir_all("/tmp/pit-cargo-git");
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "cargo new (vcs git): {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ))
    }
}
