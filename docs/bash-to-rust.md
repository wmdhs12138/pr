# Exploration: Replace proot-distro.sh with Rust Binary

Date: 2026-04-14
Status: Exploration (pre-Phase 6)
Trigger: mksh R59 on Android lacks associative arrays; bash cannot be exec'd from `untrusted_app` SELinux context.

## Problem Statement

The app process runs under `u:r:untrusted_app:s0` SELinux domain. It can `execve()` `/system/bin/sh` (mksh) but NOT bash from nativeLibraryDir — the process is killed with SIGSYS (exit 159). mksh R59 (2020/10/31) lacks associative arrays (`typeset -A` silently corrupts data), `mapfile`, process substitution, and `${!var}` indirect expansion.

Porting proot-distro.sh (3079 lines) to mksh is possible but requires replacing all associative arrays with `eval`-based flat-variable lookups — fragile, hard to test, and hard to maintain.

## Current Script Structure

### Function Breakdown

| Function | Lines | Responsibility | Bash dependency |
|---|---|---|---|
| `command_install` | 371 | Parse args, source plugin, download tarball, extract, write config | Assoc arrays, `<<<`, `eval` |
| `command_login` | 596 | Parse args, build proot argv, setup env/bind mounts, exec proot | Assoc arrays, indexed arrays, `mapfile`, `local -a`, `<<<` |
| `setup_fake_sysdata` | 244 | Write fake /proc files (loadavg, stat, uptime, version, vmstat) | None — pure file I/O |
| `command_backup` | 150 | Tar rootfs into backup archive | Minimal |
| `run_proot_cmd` | 146 | Construct proot invocation with CPU emulation args | String building only |
| `command_restore` | 90 | Extract backup archive to rootfs | Minimal |
| `command_rename` | 146 | Rename distro, update symlinks | Assoc arrays, `<<<` |
| `command_copy` | 157 | Copy rootfs between distros | `<<<` |
| `command_remove` | 77 | Delete rootfs directory | None |
| `command_list` | 79 | Iterate plugins, show distro info | Assoc arrays, `${!array[@]}` |
| `command_reset` | 62 | Wipe and reinstall rootfs | Assoc arrays |
| `command_clear_cache` | 42 | Delete download cache | None (already mksh-safe) |
| Help functions (7) | ~170 | Print usage text | None |
| Init/self_initialize | ~84 | Validate environment, detect arch | None |
| download_file | ~58 | HTTP download with retry | None |
| msg/color setup | ~100 | Terminal color output | None |
| Plugin loading | ~30 | Source plugins, populate arrays | Assoc arrays (the core problem) |

### Where Bash-isms Concentrate

The associative arrays (`declare -A`) are the central data model:

```
SUPPORTED_DISTRIBUTIONS["alpine"] = "Alpine Linux"
TARBALL_URL["aarch64"] = "https://easycli.sh/..."
TARBALL_SHA256["aarch64"] = "2bdfb..."
SUPPORTED_DISTRIBUTIONS_COMMENTS["alpine"] = "Regular release v3.23.3"
```

These are read/written in ~50 locations across the script. Under mksh R59, `typeset -A` silently fails and treats arrays as indexed — `SUPPORTED_DISTRIBUTIONS["alpine"]` and `SUPPORTED_DISTRIBUTIONS["ubuntu"]` map to the same slot, producing wrong results.

## Three Approaches

### A. Full Rust Rewrite

Replace proot-distro.sh entirely with a Rust binary (`pr-cli`).

```
pr-cli install alpine     # replaces: proot-distro.sh install alpine
pr-cli login alpine       # replaces: proot-distro.sh login alpine
pr-cli list               # replaces: proot-distro.sh list
```

**Pros:**
- Type-safe, testable, no eval hacks
- No shell compatibility concerns
- Better error handling, logging, progress reporting
- Can call proot directly via `exec(2)` from Rust
- Single binary, no template substitution needed

**Cons:**
- Higher initial effort (rewrite ~3000 lines of shell logic)
- Loses easy upstream tracking with termux/proot-distro
- APK size +2-5MB for Rust binary
- Must handle `distro_setup()` hooks differently
- Must be in nativeLibDir as `libpr-cli.so` (same W^X workaround as proot/bash)

### B. Hybrid: Rust Core + Thin Shell Wrapper

Rust handles business logic (plugin parsing, install orchestration, login command construction). A thin `/system/bin/sh` wrapper stays for glue.

**Pros:**
- Focused scope, less risk
- Shell wrapper can stay mksh-compatible (it's thin)
- Core logic benefits from Rust type safety

**Cons:**
- Two languages to maintain
- IPC between Rust and shell adds complexity
- Doesn't fully eliminate the mksh problem — the wrapper still needs to exist

### C. Full mksh Port (Current Plan)

Rewrite all bash-isms in the 3079-line shell script using `eval`-based flat variables.

**Pros:**
- No new language/toolchain dependency
- No APK size increase
- Closest to upstream proot-distro

**Cons:**
- Fragile — `eval`-based associative array simulation is error-prone
- Hard to test — quoting bugs surface only at runtime on device
- Hard to maintain — future upstream changes need manual mksh conversion
- mksh R59 has other subtle differences (here-doc temp file requirements, array key quoting)

## Analysis: Rust Feasibility

### What the shell script actually does

proot-distro.sh is a **task orchestrator**. It does not contain complex algorithms. Each command is a sequence of:

1. Parse CLI arguments
2. Read plugin metadata (distro name, tarball URLs, SHA256 hashes)
3. Download tarball (via wget/curl)
4. Extract tarball (via tar)
5. Write config files (passwd, group, environment, resolv.conf)
6. Generate fake /proc data (static files)
7. Construct proot command line (lots of bind mounts, env vars)
8. `exec` proot

All of these are straightforward in Rust.

### Plugin format change

Current (shell):
```sh
DISTRO_NAME="Alpine Linux"
DISTRO_COMMENT="Regular release v3.23.3"
TARBALL_URL_aarch64="https://easycli.sh/..."
TARBALL_SHA256_aarch64="2bdfb..."
```

This is already simple key=value. Rust can parse it with a trivial parser — no need for a full shell interpreter.

### The `distro_setup()` problem

8 plugins have post-install shell functions:
- archlinux: init pacman keyring, populate archlinux-keyring
- artix: same as archlinux
- debian: setup locales
- fedora: setup dnf, install core packages
- manjaro: same as archlinux
- opensuse: setup zypper
- trisquel: setup locales
- ubuntu: setup locales

Options:
1. **Convert to Rust** — implement each as Rust code. Most are just `Command::new("chroot").arg("locale-gen")` etc.
2. **Keep as shell fragments** — each plugin ships a `setup.sh` that Rust invokes via `/system/bin/sh` in the proot context
3. **Skip for v1** — distros work for basic usage without setup hooks

Option 2 is the most pragmatic — `distro_setup()` runs inside the proot chroot where `/system/bin/sh` can execute anything.

### Rust dependencies

Minimal set:
- `clap` or `lexopt` — CLI argument parsing
- `sha2` — SHA256 verification
- `reqwest` (or shell out to busybox wget) — HTTP downloads
- `tar` + `xz2` (or shell out to busybox tar) — archive extraction
- `dirs` — path construction

Alternative: skip Rust HTTP/tar libraries and shell out to busybox wget/tar. This keeps the binary small (~500KB) but adds busybox dependency (already bundled).

### Cross-compilation

Already have NDK 27 at `/home/o/Android/Sdk/ndk/27.0.12077973/`. Cross-compiling Rust for `aarch64-linux-android`:

```sh
rustup target add aarch64-linux-android
cargo build --target aarch64-linux-android --release
```

The resulting binary goes to `jniLibs/arm64-v8a/libpr-cli.so` (same pattern as libproot.so).

### Estimated effort

| Component | Lines of Rust | Notes |
|---|---|---|
| CLI parsing (install, login, remove, list, backup, restore) | ~200 | With `clap` derive |
| Plugin config parser | ~80 | Simple key=value |
| Download with retry | ~60 | Or shell out to wget |
| Extract tarball | ~30 | Shell out to tar |
| Fake /proc generation | ~150 | Port from setup_fake_sysdata() |
| Config file writing (passwd, group, etc.) | ~100 | Port from command_install |
| Proot command builder | ~200 | The bind mount logic is complex |
| Exec proot | ~20 | `Command::new("proot")` |
| Distro setup hooks | ~50 | Shell out via proot |
| Error handling, logging, progress | ~100 | Structured output |
| **Total** | **~1000** | vs 3079 lines of shell |

## Recommendation

**Approach A (Full Rust)** is recommended over the mksh port because:

1. **The mksh port is fragile** — eval-based associative arrays are a ticking time bomb. A quoting bug in one `eval` statement can silently corrupt distro names or tarball URLs. On-device debugging of shell eval chains is painful.

2. **The shell script is mostly plumbing** — there's minimal "shell magic" that can't be expressed in Rust. The fake /proc generation is just writing files. The install command is download + extract + write config. The login command is building an argv.

3. **Rust on Android arm64 is standard** — we already have the NDK toolchain. Cross-compilation is one `cargo build` command.

4. **The plugin format is already close** — we already converted `TARBALL_URL['aarch64']` to `TARBALL_URL_aarch64`. Converting to a config file is trivial.

5. **Future-proof** — a Rust binary is easier to extend (progress bars, parallel downloads, built-in terminal, etc.) than a 3000-line shell script.

### Suggested Plugin Format

```
# alpine.plugin
name = "Alpine Linux"
comment = "Regular release v3.23.3"

[ tarball.aarch64 ]
url = "https://easycli.sh/proot-distro/alpine-aarch64-pd-v4.37.0.tar.xz"
sha256 = "2bdfb03eae53e6163695f4cd3b86e67ddca78466c879a140e069b1263150599b"

[ tarball.arm ]
url = "https://easycli.sh/proot-distro/alpine-arm-pd-v4.37.0.tar.xz"
sha256 = "0d1bc9bb24f1efd3a95e22e04e3590f4adfb0ff1ff39bbc82281ccf12fc0916d"
```

Or simply keep the current key=value format and parse with Rust — no format change needed for v1.

### Proposed Task Breakdown

If we proceed with Rust:

1. **Rust project setup** — `src/pr-cli/` with Cargo.toml, cross-compilation config, NDK linker
2. **Plugin config parser** — read key=value files, populate structs
3. **CLI interface** — install, login, remove, list, backup, restore subcommands
4. **command_install** — download, verify, extract, write configs
5. **command_login** — build proot argv, exec
6. **setup_fake_sysdata** — generate /proc files
7. **command_list** — iterate plugins, display info
8. **command_remove/reset/backup/restore/copy** — remaining commands
9. **Integration testing** — same as T5.2-T5.6 but via Rust binary
10. **APK integration** — bundle as `libpr-cli.so`, update App.kt

### Open Questions

- **Download library or shell out?** reqwest adds ~1MB to binary. Shelling out to busybox wget is simpler but loses progress reporting.
- **Tar extraction or shell out?** tar+xz2 crates add ~500KB. Shelling out to busybox tar is simpler.
- **distro_setup() handling?** Shell out via proot for v1, or implement in Rust per distro?
- **How much of command_login's bind mount logic do we need?** The 596-line function has lots of edge cases for different Android versions and SELinux policies. We may be able to simplify for our target.
