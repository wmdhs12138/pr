use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plugins")
}

fn parse_fixture(name: &str) -> pr_cli::plugin::DistroPlugin {
    let path = fixtures_dir().join(format!("{}.sh", name));
    pr_cli::plugin::parse_plugin(&path)
        .unwrap_or_else(|e| panic!("failed to parse {}: {}", name, e))
}

#[test]
fn test_load_all_8_plugins() {
    let plugins = pr_cli::plugin::load_plugins(&fixtures_dir());
    assert_eq!(
        plugins.len(),
        8,
        "expected 8 plugins, got {}",
        plugins.len()
    );

    let aliases: Vec<&str> = plugins.iter().map(|p| p.alias.as_str()).collect();
    assert!(aliases.contains(&"alpine"));
    assert!(aliases.contains(&"ubuntu"));
    assert!(aliases.contains(&"debian"));
    assert!(aliases.contains(&"archlinux"));
}

#[test]
fn test_plugins_sorted_by_alias() {
    let plugins = pr_cli::plugin::load_plugins(&fixtures_dir());
    let aliases: Vec<&str> = plugins.iter().map(|p| p.alias.as_str()).collect();
    let mut sorted = aliases.clone();
    sorted.sort();
    assert_eq!(aliases, sorted);
}

#[test]
fn test_alpine() {
    let p = parse_fixture("alpine");
    assert_eq!(p.alias, "alpine");
    assert_eq!(p.name, "Alpine Linux");
    assert_eq!(p.comment.as_deref(), Some("Regular release v3.23.3."));
    assert!(!p.has_setup);
    assert_eq!(p.tarballs.len(), 5);
    assert!(p.tarballs.contains_key("aarch64"));
    assert!(p.tarballs.contains_key("arm"));
    assert!(p.tarballs.contains_key("i686"));
    assert!(p.tarballs.contains_key("riscv64"));
    assert!(p.tarballs.contains_key("x86_64"));
    assert!(p.tarballs["aarch64"].url.contains("alpine-aarch64"));
    assert!(!p.tarballs["aarch64"].sha256.is_empty());
}

#[test]
fn test_archlinux() {
    let p = parse_fixture("archlinux");
    assert_eq!(p.alias, "archlinux");
    assert_eq!(p.name, "Arch Linux");
    assert!(p.comment.is_some());
    assert!(p.has_setup);
    assert_eq!(p.tarballs.len(), 4);
    assert!(p.tarballs.contains_key("aarch64"));
    assert!(p.tarballs.contains_key("arm"));
    assert!(p.tarballs.contains_key("i686"));
    assert!(p.tarballs.contains_key("x86_64"));
}

#[test]
fn test_debian() {
    let p = parse_fixture("debian");
    assert_eq!(p.alias, "debian");
    assert_eq!(p.name, "Debian (trixie)");
    assert_eq!(p.comment.as_deref(), Some("Stable release."));
    assert!(p.has_setup);
    assert_eq!(p.tarballs.len(), 4);
    assert!(p.tarballs.contains_key("aarch64"));
    assert!(p.tarballs.contains_key("arm"));
    assert!(p.tarballs.contains_key("i686"));
    assert!(p.tarballs.contains_key("x86_64"));
    assert!(p.tarballs["aarch64"].url.contains("debian-trixie-aarch64"));
}

#[test]
fn test_fedora() {
    let p = parse_fixture("fedora");
    assert_eq!(p.alias, "fedora");
    assert_eq!(p.name, "Fedora");
    assert!(p.comment.is_some());
    assert!(p.has_setup);
    assert_eq!(p.tarballs.len(), 2);
    assert!(p.tarballs.contains_key("aarch64"));
    assert!(p.tarballs.contains_key("x86_64"));
}

#[test]
fn test_manjaro() {
    let p = parse_fixture("manjaro");
    assert_eq!(p.alias, "manjaro");
    assert_eq!(p.name, "Manjaro");
    assert!(p.comment.is_some());
    assert!(p.has_setup);
    assert_eq!(p.tarballs.len(), 1);
    assert!(p.tarballs.contains_key("aarch64"));
}

#[test]
fn test_opensuse() {
    let p = parse_fixture("opensuse");
    assert_eq!(p.alias, "opensuse");
    assert_eq!(p.name, "OpenSUSE");
    assert!(p.comment.is_some());
    assert!(p.has_setup);
    assert_eq!(p.tarballs.len(), 2);
    assert!(p.tarballs.contains_key("aarch64"));
    assert!(p.tarballs.contains_key("x86_64"));
}

#[test]
fn test_rockylinux() {
    let p = parse_fixture("rockylinux");
    assert_eq!(p.alias, "rockylinux");
    assert_eq!(p.name, "Rocky Linux");
    assert!(p.comment.is_some());
    assert!(!p.has_setup);
    assert_eq!(p.tarballs.len(), 2);
    assert!(p.tarballs.contains_key("aarch64"));
    assert!(p.tarballs.contains_key("x86_64"));
}

#[test]
fn test_ubuntu() {
    let p = parse_fixture("ubuntu");
    assert_eq!(p.alias, "ubuntu");
    assert_eq!(p.name, "Ubuntu (25.10)");
    assert!(p.comment.is_some());
    assert!(p.has_setup);
    assert_eq!(p.tarballs.len(), 3);
    assert!(p.tarballs.contains_key("aarch64"));
    assert!(p.tarballs.contains_key("arm"));
    assert!(p.tarballs.contains_key("x86_64"));
    assert!(p.tarballs["aarch64"]
        .url
        .contains("ubuntu-questing-aarch64"));
}

#[test]
fn test_all_tarballs_have_sha256() {
    let plugins = pr_cli::plugin::load_plugins(&fixtures_dir());
    for p in &plugins {
        for (arch, tb) in &p.tarballs {
            assert!(
                !tb.sha256.is_empty(),
                "{}: missing sha256 for arch {}",
                p.alias,
                arch
            );
            assert_eq!(
                tb.sha256.len(),
                64,
                "{}: invalid sha256 length for arch {}: {}",
                p.alias,
                arch,
                tb.sha256
            );
        }
    }
}

#[test]
fn test_all_tarballs_have_valid_url() {
    let plugins = pr_cli::plugin::load_plugins(&fixtures_dir());
    for p in &plugins {
        for (arch, tb) in &p.tarballs {
            assert!(
                tb.url.starts_with("https://"),
                "{}: invalid URL for arch {}: {}",
                p.alias,
                arch,
                tb.url
            );
            assert!(
                tb.url.ends_with(".tar.xz"),
                "{}: unexpected extension for arch {}: {}",
                p.alias,
                arch,
                tb.url
            );
        }
    }
}

#[test]
fn test_has_setup_distribution() {
    let with_setup = [
        "archlinux",
        "debian",
        "fedora",
        "manjaro",
        "opensuse",
        "ubuntu",
    ];
    let without_setup = ["alpine", "rockylinux"];

    for name in &with_setup {
        let p = parse_fixture(name);
        assert!(p.has_setup, "expected {} to have distro_setup", name);
    }
    for name in &without_setup {
        let p = parse_fixture(name);
        assert!(!p.has_setup, "expected {} to NOT have distro_setup", name);
    }
}

#[test]
fn test_display_format() {
    let p = parse_fixture("alpine");
    let s = format!("{}", p);
    assert!(s.contains("Alpine Linux"));
    assert!(s.contains("Regular release v3.23.3."));
    assert!(s.contains("aarch64"));
    assert!(s.contains("setup:    no"));
}
