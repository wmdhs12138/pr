use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::shared::*;

const PIT_BIN: &str = "/tmp/pit";

const TEST_BINARY: &[u8] = include_bytes!(
    "../../proot-integration-test/target/aarch64-linux-android/release/proot-integration-test"
);

struct TapResult {
    passed: usize,
    failed: usize,
    skipped: usize,
    failures: Vec<String>,
}

fn deploy_binary(cache_dir: &str) -> Result<(), String> {
    let bin_path = format!("{}/pit", cache_dir);
    fs::write(&bin_path, TEST_BINARY).map_err(|e| format!("write {}: {}", bin_path, e))?;
    fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("chmod {}: {}", bin_path, e))?;
    Ok(())
}

fn run_proot_command(rootfs: &str, cmd: &str) -> Result<String, String> {
    let proot = get_native_proot();
    let args = build_proot_args(rootfs, false, false, &[]);
    let runtime_env = build_proot_runtime_env();
    let child_env = build_proot_child_env();

    let full_args = {
        let mut a = args;
        a.push("/bin/sh".to_string());
        a.push("-c".to_string());
        a.push(cmd.to_string());
        a
    };

    let mut command = Command::new(&proot);
    command.arg0("proot");
    for (k, v) in &runtime_env {
        command.env(k, v);
    }
    for (k, v) in &child_env {
        command.env(k, v);
    }
    let output = command
        .args(&full_args)
        .output()
        .map_err(|e| format!("proot exec: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let stderr_trim = stderr.trim();
        let stdout_trim = stdout.trim();
        let detail = if stdout_trim.is_empty() {
            stderr_trim.to_string()
        } else {
            format!("{}\n{}", stderr_trim, stdout_trim)
        };
        return Err(format!(
            "exit={}: {}",
            output.status.code().unwrap_or(-1),
            detail
        ));
    }

    Ok(stdout)
}

fn parse_tap(tap_output: &str) -> TapResult {
    let mut result = TapResult {
        passed: 0,
        failed: 0,
        skipped: 0,
        failures: Vec::new(),
    };

    for line in tap_output.lines() {
        if line.is_empty() {
            continue;
        }
        if line.starts_with("ok ") {
            if line.contains("# SKIP") {
                result.skipped += 1;
            } else {
                result.passed += 1;
            }
            let name = line
                .trim_start_matches("ok ")
                .split_once(' ')
                .map(|(_, n)| n.to_string())
                .unwrap_or_default();
            println!(
                "  {} {}{}",
                "\x1b[32m✓\x1b[0m",
                name,
                if line.contains("# SKIP") {
                    " \x1b[33m(SKIP)\x1b[0m".to_string()
                } else {
                    "".to_string()
                }
            );
        } else if line.starts_with("not ok ") {
            result.failed += 1;
            let name = line
                .trim_start_matches("not ok ")
                .split_once(' ')
                .map(|(_, n)| n.to_string())
                .unwrap_or_default();
            result.failures.push(name.clone());
            println!("  {} {}", "\x1b[31m✗\x1b[0m", name);
        } else {
            println!("    {}", line);
        }
    }

    result
}

fn run_proot_streaming(rootfs: &str, cmd: &str) -> Result<(), String> {
    let proot = get_native_proot();
    let args = build_proot_args(rootfs, false, false, &[]);
    let runtime_env = build_proot_runtime_env();
    let child_env = build_proot_child_env();

    let full_args = {
        let mut a = args;
        a.push("/bin/sh".to_string());
        a.push("-c".to_string());
        a.push(cmd.to_string());
        a
    };

    let mut child = Command::new(&proot);
    child.arg0("proot");
    for (k, v) in &runtime_env {
        child.env(k, v);
    }
    for (k, v) in &child_env {
        child.env(k, v);
    }
    let mut child = child
        .args(&full_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("proot spawn: {}", e))?;

    use std::io::{BufRead, BufReader};
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    for line in stdout_reader.lines() {
        let line = line.unwrap_or_default();
        if !line.is_empty() {
            println!("  {}", line);
        }
    }

    for line in stderr_reader.lines() {
        let line = line.unwrap_or_default();
        if !line.is_empty() {
            println!("  {}", line);
        }
    }

    let status = child.wait().map_err(|e| format!("proot wait: {}", e))?;
    if !status.success() {
        return Err(format!("exit={}", status.code().unwrap_or(-1)));
    }

    Ok(())
}

fn detect_package_manager(rootfs: &str) -> Result<&'static str, String> {
    let mut checked = Vec::new();
    for name in &["sbin/apk", "usr/sbin/apk", "usr/bin/apk"] {
        let path = format!("{}/{}", rootfs, name);
        checked.push(path.clone());
        if Path::new(&path).exists() {
            return Ok("apk");
        }
    }
    for name in &["usr/bin/apt", "usr/bin/apt-get"] {
        let path = format!("{}/{}", rootfs, name);
        checked.push(path.clone());
        if Path::new(&path).exists() {
            return Ok("apt");
        }
    }
    Err(format!(
        "no supported package manager found (apk or apt)\n  rootfs={}\n  checked={:?}",
        rootfs, checked
    ))
}

fn install_tools(rootfs: &str, pkg_mgr: &str) -> Result<(), String> {
    msg_status("Installing tools...");

    let cmd = match pkg_mgr {
        "apk" => "apk update 2>&1 && apk add --no-progress vim gcc rust cargo git 2>&1",
        "apt" => "apt-get update -qq 2>&1 && apt-get install -y -qq vim gcc rustc cargo git 2>&1",
        _ => return Err(format!("unknown package manager: {}", pkg_mgr)),
    };

    run_proot_streaming(rootfs, cmd)
}

fn run_test_binary(rootfs: &str, suite: &str) -> Result<TapResult, String> {
    msg_status(&format!("Running test suite: {}...", suite));

    let cmd = format!("{} {} 2>&1", PIT_BIN, suite);
    let output = run_proot_command(rootfs, &cmd)?;

    let result = parse_tap(&output);
    Ok(result)
}

pub fn command_test(distro: &str, suite: Option<&str>, verbose: bool) -> Result<(), String> {
    let rootfs = format!("{}/{}", get_installed_rootfs_dir(), distro);

    if !Path::new(&rootfs).is_dir() {
        msg_error(&format!("distribution '{}' is not installed.", distro));
        return Err("not installed".to_string());
    }

    println!();
    msg_status(&format!("Testing distro: {}", distro));
    println!();

    if verbose {
        println!("  Rootfs: {}", rootfs);
        println!();
    }

    let suites = [
        "distro", "clone", "readlink", "gcc", "rust", "git", "pipe", "general",
    ];
    let target_suites: Vec<&str> = match suite {
        Some(s) => {
            if suites.contains(&s) {
                vec![s]
            } else {
                return Err(format!(
                    "unknown suite '{}'. Available: {}",
                    s,
                    suites.join(", ")
                ));
            }
        }
        None => suites.to_vec(),
    };

    let needs_tools = target_suites
        .iter()
        .any(|s| ["distro", "gcc", "rust", "git"].contains(s));
    if needs_tools {
        let has_tools = Path::new(&format!("{}/usr/bin/rustc", rootfs)).exists()
            || Path::new(&format!("{}/usr/local/bin/rustc", rootfs)).exists();
        if !has_tools {
            let pkg_mgr = detect_package_manager(&rootfs)?;
            install_tools(&rootfs, pkg_mgr)?;
            println!();
        }
    }

    let cache_dir = std::env::var("PROOT_TMP_DIR")
        .or_else(|_| std::env::var("TMPDIR"))
        .unwrap_or_else(|_| format!("{}/tmp", get_prefix()));

    msg_status("Deploying test binary...");
    deploy_binary(&cache_dir)?;

    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut total_skipped = 0;
    let mut all_failures: Vec<String> = Vec::new();

    for suite_name in &target_suites {
        match run_test_binary(&rootfs, suite_name) {
            Ok(result) => {
                total_passed += result.passed;
                total_failed += result.failed;
                total_skipped += result.skipped;
                all_failures.extend(result.failures);
            }
            Err(e) => {
                total_failed += 1;
                all_failures.push(format!("{}: {}", suite_name, e));
                println!("  {} {} — {}", "\x1b[31m✗\x1b[0m", suite_name, e);
            }
        }
        println!();
    }

    let total = total_passed + total_failed + total_skipped;
    println!(
        "\x1b[1mResults: {}/{} passed, {} failed, {} skipped\x1b[0m",
        total_passed, total, total_failed, total_skipped
    );

    if !all_failures.is_empty() {
        println!("\x1b[31mFailed:\x1b[0m");
        for f in &all_failures {
            println!("  - {}", f);
        }
        println!();
        return Err("tests failed".to_string());
    }

    println!();
    Ok(())
}
