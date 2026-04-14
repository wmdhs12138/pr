use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: pr-test <test-name> [args...]");
        eprintln!(
            "Tests: selfcheck, file-io, exec-subcommand, env-vars, network, parse-plugin, proot"
        );
        std::process::exit(1);
    }

    let result = match args[1].as_str() {
        "selfcheck" => test_selfcheck(),
        "file-io" => test_file_io(),
        "exec-subcommand" => test_exec_subcommand(),
        "env-vars" => test_env_vars(),
        "network" => test_network(),
        "parse-plugin" => test_parse_plugin(),
        "proot" => test_proot(),
        _ => {
            eprintln!("Unknown test: {}", args[1]);
            std::process::exit(1);
        }
    };

    match result {
        Ok(report) => println!("{}", report),
        Err(e) => {
            eprintln!("FAIL: {}", e);
            std::process::exit(1);
        }
    }
}

type TResult = Result<String, String>;

fn ok(msg: &str) -> TResult {
    Ok(format!("PASS: {}", msg))
}

fn pass(label: &str, detail: &str) -> String {
    format!("PASS: {} — {}", label, detail)
}

fn fail(label: &str, detail: &str) -> String {
    format!("FAIL: {} — {}", label, detail)
}

/// Test 1: Basic self-check. Can this Rust binary run at all?
/// Exercises: ProcessBuilder exec from app process, stdout capture.
fn test_selfcheck() -> TResult {
    let mut lines: Vec<String> = Vec::new();

    lines.push(pass("exec", "Rust binary executed successfully"));

    lines.push(pass("pid", &format!("PID={}", std::process::id())));

    let uid = unsafe { libc::getuid() };
    lines.push(pass("uid", &format!("UID={}", uid)));

    let arch = env::consts::ARCH;
    lines.push(pass("arch", &format!("target arch={}", arch)));

    let exe = env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    lines.push(pass("exe", &format!("path={}", exe)));

    let cwd = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    lines.push(pass("cwd", &format!("dir={}", cwd)));

    lines.push(pass("args", &format!("argc={}", env::args().count())));

    Ok(lines.join("\n"))
}

/// Test 2: File I/O in app data directory.
/// Exercises: create, write, read, mkdir, rm, symlink, chmod in PREFIX dir.
fn test_file_io() -> TResult {
    let prefix =
        env::var("APP_PREFIX").unwrap_or_else(|_| "/data/data/id.or.oo.pr/files/usr".to_string());
    let test_dir = format!("{}/tmp/pr-test", prefix);
    let mut lines: Vec<String> = Vec::new();

    // mkdir
    fs::create_dir_all(&test_dir).map_err(|e| format!("mkdir {}: {}", test_dir, e))?;
    lines.push(pass("mkdir", &test_dir));

    // write
    let test_file = format!("{}/test.txt", test_dir);
    fs::write(&test_file, "hello from rust\n").map_err(|e| format!("write: {}", e))?;
    lines.push(pass("write", &test_file));

    // read
    let content = fs::read_to_string(&test_file).map_err(|e| format!("read: {}", e))?;
    if content == "hello from rust\n" {
        lines.push(pass(
            "read",
            &format!("content matches ({} bytes)", content.len()),
        ));
    } else {
        return Err(fail("read", &format!("content mismatch: {:?}", content)));
    }

    // symlink
    let link_path = format!("{}/test-link.txt", test_dir);
    let _ = fs::remove_file(&link_path);
    std::os::unix::fs::symlink(&test_file, &link_path).map_err(|e| format!("symlink: {}", e))?;
    let link_target = fs::read_link(&link_path).map_err(|e| format!("readlink: {}", e))?;
    lines.push(pass(
        "symlink",
        &format!("{} -> {}", link_path, link_target.display()),
    ));

    // chmod
    fs::set_permissions(
        &test_file,
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    )
    .map_err(|e| format!("chmod: {}", e))?;
    let mode = fs::metadata(&test_file).map_err(|e| format!("stat: {}", e))?;
    let mode_val = std::os::unix::fs::PermissionsExt::mode(&mode.permissions());
    lines.push(pass("chmod", &format!("mode={:o}", mode_val)));

    // list dir
    let entries: Vec<String> = fs::read_dir(&test_dir)
        .map_err(|e| format!("readdir: {}", e))?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    lines.push(pass(
        "readdir",
        &format!("{} entries: {:?}", entries.len(), entries),
    ));

    // cleanup
    fs::remove_dir_all(&test_dir).map_err(|e| format!("rm -rf {}: {}", test_dir, e))?;
    lines.push(pass("rm-rf", &test_dir));

    Ok(lines.join("\n"))
}

/// Test 3: Fork+exec subcommands.
/// Exercises: running busybox applets (wget, tar, sha256sum), /system/bin/sh, proot.
fn test_exec_subcommand() -> TResult {
    let prefix =
        env::var("APP_PREFIX").unwrap_or_else(|_| "/data/data/id.or.oo.pr/files/usr".to_string());
    let mut lines: Vec<String> = Vec::new();

    // busybox
    let bb = format!("{}/bin/busybox", prefix);
    if Path::new(&bb).exists() {
        let out = Command::new(&bb)
            .arg("--version")
            .output()
            .map_err(|e| format!("busybox exec: {}", e))?;
        let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
        lines.push(pass(
            "exec-busybox",
            &format!("version={}, exit={}", ver, out.status.code().unwrap_or(-1)),
        ));

        // busybox wget --help
        let out = Command::new(&bb)
            .arg("wget")
            .arg("--help")
            .output()
            .map_err(|e| format!("busybox wget: {}", e))?;
        lines.push(pass(
            "exec-wget",
            &format!("exit={}", out.status.code().unwrap_or(-1)),
        ));

        // busybox sha256sum --help
        let out = Command::new(&bb)
            .arg("sha256sum")
            .arg("--help")
            .output()
            .map_err(|e| format!("busybox sha256sum: {}", e))?;
        lines.push(pass(
            "exec-sha256sum",
            &format!("exit={}", out.status.code().unwrap_or(-1)),
        ));

        // busybox tar --help
        let out = Command::new(&bb)
            .arg("tar")
            .arg("--help")
            .output()
            .map_err(|e| format!("busybox tar: {}", e))?;
        lines.push(pass(
            "exec-tar",
            &format!("exit={}", out.status.code().unwrap_or(-1)),
        ));
    } else {
        lines.push(fail("exec-busybox", &format!("not found at {}", bb)));
    }

    // /system/bin/sh
    let out = Command::new("/system/bin/sh")
        .arg("-c")
        .arg("echo hello-from-sh")
        .output()
        .map_err(|e| format!("sh exec: {}", e))?;
    let sh_out = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if sh_out == "hello-from-sh" {
        lines.push(pass("exec-sh", &format!("output='{}'", sh_out)));
    } else {
        lines.push(fail(
            "exec-sh",
            &format!("expected 'hello-from-sh', got '{}'", sh_out),
        ));
    }

    // proot
    let proot = format!("{}/bin/proot", prefix);
    if Path::new(&proot).exists() {
        let out = Command::new(&proot)
            .arg("--version")
            .output()
            .map_err(|e| format!("proot exec: {}", e))?;
        let ver = String::from_utf8_lossy(&out.stdout);
        let ver_line = ver
            .lines()
            .find(|l| l.contains("proot"))
            .unwrap_or("unknown");
        lines.push(pass(
            "exec-proot",
            &format!("exit={}", out.status.code().unwrap_or(-1)),
        ));
    } else {
        lines.push(fail("exec-proot", "not found"));
    }

    // bash (expected to fail from app process)
    let bash = format!("{}/bin/bash", prefix);
    if Path::new(&bash).exists() {
        let out = Command::new(&bash)
            .arg("--version")
            .output()
            .map_err(|e| format!("bash exec error: {}", e))?;
        let code = out.status.code().unwrap_or(-1);
        if code == 0 {
            lines.push(pass("exec-bash", "exit=0 (unexpected!)"));
        } else {
            lines.push(format!(
                "KNOWN: exec-bash — exit={} (SIGSYS expected from untrusted_app)",
                code
            ));
        }
    }

    Ok(lines.join("\n"))
}

/// Test 4: Environment variable propagation from app process.
fn test_env_vars() -> TResult {
    let mut lines: Vec<String> = Vec::new();

    let keys = [
        "APP_PREFIX",
        "APP_HOME",
        "APP_PACKAGE",
        "PROOT_NO_SECCOMP",
        "HOME",
        "TERM",
        "TMPDIR",
        "PATH",
        "ANDROID_ROOT",
        "ANDROID_DATA",
        "EXTERNAL_STORAGE",
    ];

    for key in &keys {
        match env::var(key) {
            Ok(val) => lines.push(pass("env", &format!("{}={}", key, val))),
            Err(_) => lines.push(format!("INFO: env — {} (not set)", key)),
        }
    }

    // count total env
    let all_vars: Vec<String> = env::vars().map(|(k, _)| k).collect();
    lines.push(pass(
        "env-count",
        &format!("{} total env vars", all_vars.len()),
    ));

    Ok(lines.join("\n"))
}

/// Test 5: Network access — download a small file via busybox wget.
fn test_network() -> TResult {
    let prefix =
        env::var("APP_PREFIX").unwrap_or_else(|_| "/data/data/id.or.oo.pr/files/usr".to_string());
    let tmpdir = format!("{}/tmp", prefix);
    fs::create_dir_all(&tmpdir).ok();

    let bb = format!("{}/bin/busybox", prefix);
    let outfile = format!("{}/pr-test-download", tmpdir);
    let _ = fs::remove_file(&outfile);

    let url = "https://easycli.sh/proot-distro/alpine-aarch64-pd-v4.37.0.tar.xz";

    // Test 1: busybox wget — download first 1MB via range header
    // (busybox wget doesn't support range, so just test connectivity with a HEAD-like approach)
    // Actually, test by downloading a small known file. Use busybox wget on a small URL.
    let test_url = "https://easycli.sh/";

    let out = Command::new(&bb)
        .args(["wget", "-q", "-O", &outfile, test_url])
        .output()
        .map_err(|e| format!("wget exec: {}", e))?;

    let mut lines: Vec<String> = Vec::new();

    let code = out.status.code().unwrap_or(-1);
    if code == 0 {
        let meta = fs::metadata(&outfile);
        match meta {
            Ok(m) => {
                lines.push(pass(
                    "network-wget",
                    &format!("downloaded {} bytes from {}", m.len(), test_url),
                ));
            }
            Err(e) => lines.push(fail("network-wget", &format!("file check: {}", e))),
        }
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        lines.push(fail(
            "network-wget",
            &format!("exit={}, stderr={}", code, stderr),
        ));
    }

    // Test 2: DNS resolution
    let out = Command::new(&bb)
        .args(["wget", "-q", "-O", "/dev/null", "https://dns.google/"])
        .output()
        .map_err(|e| format!("dns test: {}", e))?;
    let code = out.status.code().unwrap_or(-1);
    if code == 0 {
        lines.push(pass("network-dns", "DNS resolution works"));
    } else {
        lines.push(fail("network-dns", &format!("exit={}", code)));
    }

    // cleanup
    let _ = fs::remove_file(&outfile);

    Ok(lines.join("\n"))
}

/// Test 6: Parse a plugin config file (key=value format).
fn test_parse_plugin() -> TResult {
    let prefix =
        env::var("APP_PREFIX").unwrap_or_else(|_| "/data/data/id.or.oo.pr/files/usr".to_string());
    let plugin_dir = format!("{}/etc/proot-distro", prefix);
    let mut lines: Vec<String> = Vec::new();

    let entries = fs::read_dir(&plugin_dir)
        .map_err(|e| format!("readdir {}: {}", plugin_dir, e))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name().to_string_lossy().ends_with(".sh")
                || e.file_name().to_string_lossy().ends_with(".plugin")
        })
        .collect::<Vec<_>>();

    lines.push(pass(
        "plugin-dir",
        &format!("found {} plugins", entries.len()),
    ));

    for entry in &entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let content = fs::read_to_string(entry.path()).unwrap_or_default();

        let mut distro_name = String::new();
        let mut url_count = 0;
        let mut sha_count = 0;

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some(val) = line.strip_prefix("DISTRO_NAME=") {
                distro_name = val.trim_matches('"').to_string();
            }
            if line.starts_with("TARBALL_URL_") {
                url_count += 1;
            }
            if line.starts_with("TARBALL_SHA256_") {
                sha_count += 1;
            }
        }

        lines.push(pass(
            "plugin",
            &format!(
                "{}: name='{}', {} URLs, {} SHA256s",
                name, distro_name, url_count, sha_count
            ),
        ));
    }

    Ok(lines.join("\n"))
}

/// Test 7: Proot — can we actually run proot from a Rust-spawned subprocess?
fn test_proot() -> TResult {
    let prefix =
        env::var("APP_PREFIX").unwrap_or_else(|_| "/data/data/id.or.oo.pr/files/usr".to_string());
    let mut lines: Vec<String> = Vec::new();

    let proot = format!("{}/bin/proot", prefix);

    // proot --version
    let out = Command::new(&proot)
        .arg("--version")
        .output()
        .map_err(|e| format!("proot --version: {}", e))?;
    let code = out.status.code().unwrap_or(-1);
    lines.push(pass("proot-version", &format!("exit={}", code)));

    // proot echo hello (inside proot namespace)
    let out = Command::new(&proot)
        .env("PROOT_NO_SECCOMP", "1")
        .args(["/system/bin/echo", "hello-from-proot"])
        .output()
        .map_err(|e| format!("proot echo: {}", e))?;
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();

    if code == 0 && stdout == "hello-from-proot" {
        lines.push(pass("proot-exec", &format!("output='{}'", stdout)));
    } else {
        lines.push(fail(
            "proot-exec",
            &format!("exit={}, stdout='{}', stderr='{}'", code, stdout, stderr),
        ));
    }

    Ok(lines.join("\n"))
}
