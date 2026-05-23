use super::TestResult;
use std::path::Path;

fn run_sh(cmd: &str) -> Result<std::process::Output, String> {
    std::process::Command::new("/bin/sh")
        .args(["-c", cmd])
        .output()
        .map_err(|e| format!("sh: {}", e))
}

pub fn probe() -> bool {
    Path::new("/usr/bin/ssh").exists()
}

/// ssh -V writes to stderr; redirect 2>&1 so we can check the output.
pub fn test_ssh_version() -> TestResult {
    let out = run_sh("ssh -V 2>&1")?;
    let combined = String::from_utf8_lossy(&out.stdout).to_string();
    if combined.contains("OpenSSH") {
        Ok(())
    } else {
        Err(format!(
            "ssh -V: expected 'OpenSSH' in output, got: {:?}",
            combined
        ))
    }
}

/// Generate an ed25519 key pair with no passphrase, verify it succeeds.
pub fn test_ssh_keygen_ed25519() -> TestResult {
    let key = "/tmp/pit-ssh-ed25519";
    let _ = run_sh(&format!("rm -f {} {}.pub", key, key));
    let out = run_sh(&format!("ssh-keygen -t ed25519 -N '' -f {} 2>&1", key))?;
    let ok = out.status.success();
    let _ = run_sh(&format!("rm -f {} {}.pub", key, key));
    if ok {
        Ok(())
    } else {
        Err(format!(
            "ssh-keygen ed25519: {:?}",
            String::from_utf8_lossy(&out.stdout)
        ))
    }
}

/// Generate a key and read its SHA256 fingerprint with ssh-keygen -l.
pub fn test_ssh_keygen_fingerprint() -> TestResult {
    let key = "/tmp/pit-ssh-fp";
    let _ = run_sh(&format!("rm -f {} {}.pub", key, key));
    let gen = run_sh(&format!("ssh-keygen -t ed25519 -N '' -f {} 2>&1", key))?;
    if !gen.status.success() {
        let _ = run_sh(&format!("rm -f {} {}.pub", key, key));
        return Err(format!(
            "keygen for fingerprint: {:?}",
            String::from_utf8_lossy(&gen.stdout)
        ));
    }
    let out = run_sh(&format!("ssh-keygen -l -f {}.pub 2>&1", key))?;
    let combined = String::from_utf8_lossy(&out.stdout).to_string();
    let _ = run_sh(&format!("rm -f {} {}.pub", key, key));
    if out.status.success() && combined.contains("SHA256:") {
        Ok(())
    } else {
        Err(format!(
            "ssh-keygen -l: expected SHA256 fingerprint, got: {:?}",
            combined
        ))
    }
}

/// Confirm scp is present and executable.
/// scp has no -V flag; running it with no args exits non-zero but prints
/// its usage line to stderr, which is enough to confirm the binary works.
pub fn test_scp_available() -> TestResult {
    if !std::path::Path::new("/usr/bin/scp").exists() {
        return Err("scp not found at /usr/bin/scp".to_string());
    }
    // `|| true` keeps the shell exit 0 so run_sh doesn't error on the
    // expected non-zero exit from scp-with-no-args.
    let out = run_sh("scp 2>&1 || true")?;
    let combined = String::from_utf8_lossy(&out.stdout).to_string();
    if combined.contains("usage") || combined.contains("scp") {
        Ok(())
    } else {
        Err(format!("unexpected scp output: {:?}", combined))
    }
}
