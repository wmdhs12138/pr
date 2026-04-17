use super::TestResult;

pub fn probe() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn test_git_init() -> TestResult {
    let _ = std::fs::remove_dir_all("/tmp/pit-git");
    std::fs::create_dir_all("/tmp/pit-git").map_err(|e| format!("mkdir: {}", e))?;
    let out = std::process::Command::new("git")
        .args(["init"])
        .current_dir("/tmp/pit-git")
        .output()
        .map_err(|e| format!("git init: {}", e))?;
    let _ = std::fs::remove_dir_all("/tmp/pit-git");
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git init: {:?}",
            std::str::from_utf8(&out.stderr).unwrap_or("")
        ))
    }
}

pub fn test_git_config() -> TestResult {
    let out = std::process::Command::new("git")
        .args(["config", "--global", "user.name", "test"])
        .output()
        .map_err(|e| format!("git config: {}", e))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git config: {:?}",
            std::str::from_utf8(&out.stderr).unwrap_or("")
        ))
    }
}

pub fn test_cargo_new_git() -> TestResult {
    let _ = std::fs::remove_dir_all("/tmp/pit-cargo-git");
    let out = std::process::Command::new("cargo")
        .args(["new", "/tmp/pit-cargo-git"])
        .output()
        .map_err(|e| format!("cargo new: {}", e))?;
    let _ = std::fs::remove_dir_all("/tmp/pit-cargo-git");
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "cargo new (vcs git): {:?}",
            std::str::from_utf8(&out.stderr).unwrap_or("")
        ))
    }
}
