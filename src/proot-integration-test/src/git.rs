use super::TestResult;

fn run_sh(cmd: &str) -> Result<std::process::Output, String> {
    std::process::Command::new("/bin/sh")
        .args(["-c", cmd])
        .output()
        .map_err(|e| format!("sh: {}", e))
}

pub fn probe() -> bool {
    std::path::Path::new("/usr/bin/git").exists()
}

fn ensure_git_config() -> TestResult {
    let out = run_sh("mkdir -p /root/.config/git && touch /root/.config/git/config 2>&1")?;
    if !out.status.success() {
        return Err(format!(
            "mkdir git config: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or("")
        ));
    }
    Ok(())
}

pub fn test_git_init() -> TestResult {
    let _ = run_sh("rm -rf /root/pit-git 2>&1");
    run_sh("mkdir -p /root/pit-git 2>&1").map_err(|e| format!("mkdir: {}", e))?;
    ensure_git_config()?;
    let out = run_sh("cd /root/pit-git && git init 2>&1")?;
    let ls = match run_sh("ls -la /root/pit-git/.git/ 2>&1") {
        Ok(o) => std::str::from_utf8(&o.stdout).unwrap_or("").to_string(),
        Err(e) => e,
    };
    let ls2 = match run_sh("ls -la /root/pit-git/.git/refs/ 2>&1") {
        Ok(o) => std::str::from_utf8(&o.stdout).unwrap_or("").to_string(),
        Err(e) => e,
    };
    let _ = run_sh("rm -rf /root/pit-git 2>&1");
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "git init: {:?}\n.git/: {:?}\n.git/refs/: {:?}",
            std::str::from_utf8(&out.stdout).unwrap_or(""),
            ls,
            ls2
        ))
    }
}

pub fn test_git_config() -> TestResult {
    ensure_git_config()?;
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
    ensure_git_config()?;
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
