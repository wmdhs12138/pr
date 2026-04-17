# AGENTS.md

Guide for AI coding agents working on this project.

## Project Overview

**pr** — Android app that runs Linux distributions via proot (ptrace-based root).
- Android package: `id.or.oo.pr`
- proot version: `5.4.0-pr`, fake kernel: `6.17.0-pr`
- Languages: C (proot), Rust (pr-cli, test binary), Kotlin (Android app), POSIX sh (bootstrap)

## Privacy Policy

**NEVER include personal device information in code, comments, commits, or docs.**

Specifically:
- **NO device model numbers** — mask partial: `SM-XXXXX` is OK, full model is not
- **NO marketing names** — mask partial: `Galaxy X` is OK, full product name is not
- **NO device serial numbers** — mask like `ABCD12XXXX` is OK, real serial is not
- **NO adb serials** in commands, scripts, or documentation — use bare `adb`

**OK to include:**
- Manufacturer: Samsung
- Architecture: aarch64, arm64-v8a
- OS version: Android 16 (SDK 36)
- SELinux context: untrusted_app, runas_app

When writing adb commands in docs/scripts, use bare `adb` without `-s <serial>`.

## Directory Structure

```
src/proot/                  # Patched proot C source (working copy, NOT vendor/)
src/pr-cli/                 # Rust CLI (replaces proot-distro.sh)
src/proot-integration-test/ # Guest-side test binary (runs inside proot, TAP output)
src/scripts/                # Shell scripts: bootstrap.sh, plugins/
android/                    # Android APK (Kotlin + Compose + JNI)
scripts/                    # Host-side build scripts (build.sh, download-*.sh)
vendor/                     # Git submodules — READ ONLY, never modify
docs/                       # Phase documentation, important-notes.md
openspec/                   # Change management (proposal, design, tasks)
build/                      # Build artifacts (gitignored)
```

## Build Process

### 1. Build proot (C, NDK cross-compilation)

```bash
scripts/build.sh --arch=arm64
# Output: build/out/arm64/proot, build/out/arm64/loader
# Copy to android/app/src/main/jniLibs/arm64-v8a/libproot.so
# Copy to android/app/src/main/jniLibs/arm64-v8a/libproot-loader.so
```

### 2. Build pr-cli (Rust, NDK cross-compilation)

```bash
# MUST run from src/pr-cli/ (required for .cargo/config.toml resolution)
cd src/pr-cli && cargo build --target aarch64-linux-android --release
# Output: target/aarch64-linux-android/release/pr-cli (~900KB)
# Copy to android/app/src/main/jniLibs/arm64-v8a/libpr-cli.so
```

### 3. Build test binary (guest-side, cross-compiled)

```bash
# MUST run from src/proot-integration-test/
cd src/proot-integration-test && cargo build --target aarch64-linux-android --release
# Embedded into pr-cli via include_bytes! at build time
# Must rebuild pr-cli after rebuilding test binary
```

### 4. Build APK

```bash
cd android && ./gradlew assembleDebug
# Output: android/app/build/outputs/apk/debug/app-debug.apk
```

### 5. Install on device

```bash
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
# User must force-close and reopen app after install
```

### Full rebuild sequence (after changing test binary)

```bash
# 1. Build test binary
cd src/proot-integration-test && cargo build --target aarch64-linux-android --release
# 2. Build pr-cli (embeds test binary)
cd src/pr-cli && cargo build --target aarch64-linux-android --release
# 3. Copy to jniLibs (ALWAYS verify with md5sum)
cp -f src/pr-cli/target/aarch64-linux-android/release/pr-cli \
      android/app/src/main/jniLibs/arm64-v8a/libpr-cli.so
md5sum src/pr-cli/target/aarch64-linux-android/release/pr-cli \
       android/app/src/main/jniLibs/arm64-v8a/libpr-cli.so
# 4. Build APK
cd android && ./gradlew assembleDebug
# 5. Install
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
```

**IMPORTANT:** Gradle caches jniLibs — always verify the copy succeeded with `md5sum` before building the APK.

## Testing

### Host-side (pr-cli unit/integration)

```bash
cd src/pr-cli && cargo test
# 32 tests: 12 unit + 20 integration (14 plugin parsers)
```

### On-device integration tests

Run from app UI via BugReport button on each distro row, or:

```bash
adb shell run-as id.or.oo.pr files/usr/bin/pr-cli test alpine
```

**Test suites:** distro (8), clone (5), readlink (6), gcc (3), rust (3), git (3), general (5)

### Known test failures

- **rust suite**: 1/3 pass (rustc -vV works). rustc compile and cargo build fail with ENOSYS — blocked by T8.4 (clone3 syscall blocked by Android seccomp)
- **git suite**: 3/3 skipped. git binary cannot be exec'd from bionic test binary inside proot

## Code Conventions

### Working directory requirements

- `src/pr-cli/` — must be cwd for cargo build (`.cargo/config.toml`)
- `src/proot-integration-test/` — must be cwd for cargo build
- `android/` — must be cwd for gradlew

### Vendor directories are READ ONLY

All modifications go in `src/proot/` (working copy), never in `vendor/`.

### Native library disguise

All native binaries (proot, busybox, bash, pr-cli) are named `lib*.so` in jniLibs/ so Android extracts them to nativeLibraryDir where SELinux allows execve. They are standalone ELF executables.

### Environment variable contract

- `APP_PREFIX` — app files directory (e.g. `/data/data/id.or.oo.pr/files/usr`)
- `APP_HOME` — app home directory
- `APP_PACKAGE` — `id.or.oo.pr`
- `PROOT_NO_SECCOMP=1` — disables proot's own seccomp filter (not the zygote's)
- `PROOT_LOADER` — path to proot loader in nativeLibraryDir

### Shell commands inside proot

The test binary is bionic (Android). It cannot directly exec dynamic Alpine (musl) binaries inside proot — gets ENOSYS. **ALL commands that exec distro binaries must go through `/bin/sh -c`:**

```rust
// BROKEN — ENOSYS:
Command::new("apk").args(["update"]).output()

// CORRECT:
Command::new("/bin/sh").args(["-c", "apk update 2>&1"]).output()
```

### Commit style

Format: `<task-id>: <description>`
Examples: `T9.6: mark gcc suite complete`, `T8.2: fix GCC prefix resolution`
Task IDs reference `openspec/changes/initial-implementation/tasks.md`.

### Code style

- No comments unless explicitly requested
- Follow existing patterns in each file
- Use `cfg(target_os = "android")` to gate Android-specific code for host testability

## Critical Constraints

1. **targetSdk MUST be 35** — Play Store minimum. Already works via PROOT_LOADER mechanism.
2. **W^X / SELinux** — Only nativeLibraryDir allows execve. Files in app data cannot be exec'd.
3. **Zygote seccomp** blocks 12+ syscalls. proot has SIGSYS handlers — never remove them.
4. **bootstrap.sh must be POSIX sh** — runs before bash is available.
5. **ARM64 x0 clobber bug** — register x0 may be clobbered by kernel before proot reads it at SIGSYS time. All handlers use ORIGINAL register version.
6. **TLS alignment** — proot needs 64-byte PT_TLS alignment for Bionic. Build script patches this.

## OpenSpec Workflow

Tasks tracked in `openspec/changes/initial-implementation/tasks.md` with IDs like T1.1, T9.5.
Use OpenSpec skills for proposing changes, implementing tasks, and archiving.

## Key References

- `docs/important-notes.md` — Critical constraints, read first
- `docs/phase8.md` — vfork/CLONE_VM fix, link2symlink readlink fix
- `openspec/changes/initial-implementation/tasks.md` — Full task tracking (~900 lines)
