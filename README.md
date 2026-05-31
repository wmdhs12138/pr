# pr

Android app that runs Linux distributions via [proot](https://github.com/proot-me/proot) — no root, no Termux required.

## What it does

- Install and run Linux distributions (Alpine, Debian, Ubuntu, and 5 more) on any Android device
- Full package manager support: `apk`, `apt-get`
- Compile and run C, Rust, and other programs inside the guest
- targetSdk 35 — Play Store compatible

## Supported distributions

Alpine, Debian, Ubuntu, Arch Linux, Fedora, OpenSUSE, Manjaro, Rocky Linux

## How it works

**proot** uses Linux `ptrace()` to intercept syscalls and translate filesystem paths, creating a virtual root filesystem without actual root privileges.

The app bundles:
- **Patched proot** (C, ~90 source files) — handles Android-specific seccomp filters, SELinux, and W^X restrictions
- **pr-cli** (Rust, ~2K LOC) — replaces the original proot-distro.sh bash script
- **Android APK** (Kotlin + Compose) — install/login/remove UI with embedded terminal

### Android compatibility

Android enforces several restrictions on app processes:
- **W^X (Write-XOR-Execute)**: Prevents executing files in app-writable directories
- **SELinux**: Blocks certain filesystem operations
- **Zygote seccomp**: Blocks 18+ syscalls via BPF filter

Our proot fork handles all of these:
- SIGSYS handlers intercept blocked syscalls and emulate them in userspace
- The `PROOT_LOADER` mechanism uses nativeLibraryDir to bypass W^X
- Fake root (`--change-id=0:0`) makes `dpkg` and `apt-get` work without real root
- `CLONE_VM`/`CLONE_VFORK` stripping enables Rust's `cargo build` to work inside proot

## Building

### Prerequisites

- Android SDK with NDK r27c (auto-downloaded by build script)
- Rust toolchain with `aarch64-linux-android` target
- Java 17+

### Build steps

```bash
# 1. Build proot
scripts/build.sh --arch=arm64

# 2. Build test binary
cd src/proot-integration-test && cargo build --target aarch64-linux-android --release

# 3. Build pr-cli (embeds test binary)
cd src/pr-cli && cargo build --target aarch64-linux-android --release

# 4. Copy binaries to APK
cp build/out/arm64/proot android/app/src/main/jniLibs/arm64-v8a/libproot.so
cp build/out/arm64/loader android/app/src/main/jniLibs/arm64-v8a/libproot-loader.so
cp src/pr-cli/target/aarch64-linux-android/release/pr-cli \
   android/app/src/main/jniLibs/arm64-v8a/libpr-cli.so

# 5. Build APK
cd android && ./gradlew assembleDebug
```

## Testing

### On-device (requires connected Android device)

```bash
# Install Alpine or Debian via the app UI, then:
adb shell run-as id.or.oo.pr files/usr/bin/pr-cli test alpine
adb shell run-as id.or.oo.pr files/usr/bin/pr-cli test debian
```

**37 tests** across 8 suites: distro, clone, readlink, gcc, rust, git, pipe, general

### Host-side (pr-cli unit tests)

```bash
cd src/pr-cli && cargo test
```

## Project structure

```
src/proot/                  # Patched proot C source
src/pr-cli/                 # Rust CLI — install, login, remove, test, backup, etc.
src/proot-integration-test/ # Guest-side test binary (TAP output)
src/scripts/                # Distro plugins (.sh), bootstrap.sh
android/                    # Android APK (Kotlin + Jetpack Compose + JNI)
scripts/                    # Host-side build scripts
vendor/                     # Git submodules (upstream proot, termux-proot)
docs/                       # Technical documentation
```

## Documentation

| Document | Description |
|----------|-------------|
| [`docs/important-notes.md`](docs/important-notes.md) | Critical constraints, seccomp handlers — read first |
| [`docs/proot-improvement.md`](docs/proot-improvement.md) | Our proot fork vs upstream and Termux (27 sections) |
| [`docs/phase7-targetSdk35.md`](docs/phase7-targetSdk35.md) | How targetSdk 35 works (PROOT_LOADER mechanism) |
| [`docs/phase8-rust-support.md`](docs/phase8-rust-support.md) | vfork/CLONE_VM fix, link2symlink readlink fix |
| [`docs/phase9-integration-tests.md`](docs/phase9-integration-tests.md) | Integration test suite (37/37 pass) |

## Credits

- [proot](https://github.com/proot-me/proot) — upstream proot v5.4.0 (GPL-2.0)
- [termux-proot](https://github.com/termux/termux-proot) — Termux's proot fork with Android patches (GPL-2.0)
- [proot-distro](https://github.com/termux/termux-packages/tree/master/packages/proot-distro) — distro plugins (GPL-3.0)

## License

This project uses different licenses for different components. See [LICENSE](LICENSE) for details.

| Component | License | Why |
|-----------|---------|-----|
| `src/proot/` (C) | **GPL-2.0-or-later** | Derivative of proot (GPL-2.0) and termux-proot (GPL-2.0) |
| `src/scripts/plugins/` | **GPL-3.0-or-later** | Derivative of termux-proot-distro plugins (GPL-3.0) |
| `src/pr-cli/` (Rust) | **MIT** | Clean reimplementation of proot-distro CLI |
| `src/proot-integration-test/` (Rust) | **MIT** | Standalone test binary |
| `android/` (Kotlin) | **MIT** | App UI and JNI bridge |
| `scripts/`, `docs/` | **MIT** | Build scripts and documentation |
