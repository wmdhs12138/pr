# Build & Test Scripts

All scripts live in `scripts/` and should be run from the project root.

## `scripts/build.sh` — Build proot for Android

Cross-compiles proot using the Android NDK. Downloads NDK r27c automatically if not found.

```
scripts/build.sh [OPTIONS]

Options:
  --arch=ARCH      Target: arm64, arm, or all (default: all)
  --ndk-path=PATH  Path to existing NDK (skips download)
  --skip-talloc    Skip libtalloc build (use existing)
  --skip-ndk       Skip NDK setup (already configured)
  --clean          Clean build output
  -v, --verbose    Verbose output
  -h, --help       Show help

Environment:
  NDK_PATH         Path to Android NDK (overrides --ndk-path)
  PROOT_NDK_DIR    Directory for NDK download (default: build/ndk/)

Output: build/out/<arch>/proot
```

### Quick start

```bash
scripts/build.sh --arch=arm64 --skip-ndk --skip-talloc
```

### Build pipeline

1. Set up NDK standalone toolchain (API 28)
2. Build libtalloc as static library (from `vendor/samba/`)
3. Build proot with NDK clang (statically linked)
4. Fix TLS alignment (Android Bionic requires 64-byte)
5. Verify binary (ELF, static, correct arch)

## `scripts/download-busybox.sh` — Download static busybox

Downloads Alpine's `busybox-static` package and extracts the binary.

```
scripts/download-busybox.sh [OPTIONS]

Options:
  --arch=ARCH       Target architecture (default: aarch64)
  -f, --force       Re-download even if binary exists
  --verify-only     Only verify existing binary, skip download
  -h, --help        Show help

Output: build/assets/arm64-v8a/busybox
```

Source: Alpine Linux `busybox-static` v1.37.0-r14 (GPL-2.0-only)

## `scripts/download-bash.sh` — Download static bash

Downloads a statically-linked bash binary from robxu9/bash-static.

```
scripts/download-bash.sh [OPTIONS]

Options:
  -f, --force       Re-download even if binary exists
  --verify-only     Only verify existing binary, skip download
  -h, --help        Show help

Output: build/assets/arm64-v8a/bash
```

Source: robxu9/bash-static v5.2.015 (GPL-3.0-or-later)

## `scripts/test-push.sh` — Push test environment to device

Pushes built binaries and scripts to a connected Android device via adb for manual testing.

```
scripts/test-push.sh [ACTION]

Actions:
  (none)     Push files only
  setup      Push + run bootstrap.sh on device
  test       Push + bootstrap + run proot-distro list
  shell      Push + bootstrap + start interactive bash

Prerequisites:
  - adb in PATH, device connected
  - proot built:     scripts/build.sh --arch=arm64
  - busybox ready:   scripts/download-busybox.sh
  - bash ready:      scripts/download-bash.sh

Device path: /data/local/tmp/pr-test/usr/
```

## `scripts/test-setup.sh` — On-device test setup

POSIX shell script that runs **on the Android device** (not the host). Sets up the test directory structure, installs busybox applet symlinks, and copies scripts.

```
adb shell sh /data/local/tmp/test-setup.sh
```

This is a legacy script. `scripts/test-push.sh` with the `setup` action is preferred.
