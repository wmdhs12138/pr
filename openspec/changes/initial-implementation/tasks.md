# Tasks: Initial Implementation

## Phase 1 — Patched proot Binary

- [x] **T1.1** Copy new files from termux-proot into src/proot/ (working copy)
  - `src/tracee/seccomp.c`, `src/tracee/seccomp.h` (P0)
  - `src/tracee/statx.c`, `src/tracee/statx.h` (P1)
  - `src/path/f2fs-bug.c`, `src/path/f2fs-bug.h` (P2)
  - `src/loader/assembly-arm64.h` (P0) — identical in both, skipped
  - `src/extension/ashmem_memfd/` (P1)
  - `src/extension/link2symlink/` (P0) — overwrote vanilla with termux version (387-line diff)
  - `src/extension/fix_symlink_size/` (P2)
  - `src/extension/sysvipc/` (P2)
  - `src/extension/hidden_files/` (P2)
  - `src/extension/mountinfo/` (P2)
  - `src/extension/port_switch/` (P2)
  - `src/loader/loader-info.awk` (additional, new in termux-proot)

- [x] **T1.2** Merge modifications to existing proot source files (in src/proot/src/)
  - Copied from vendor/termux-proot/ (Approach A: direct copy of termux versions)
  - `src/arch.h` — ARM64 definitions, POKEDATA workaround, loader addresses, aarch32 support
  - `src/tracee/tracee.h` — new struct fields (killall_on_exit, seccomp state, aarch32, chain, pokedata workaround)
  - `src/tracee/mem.c` — ARM64 POKEDATA workaround assembly stub, ptrace_pokedata_or_via_stub()
  - `src/tracee/event.c` — unified seccomp-aware event loop, SIGSYS handler dispatch, kill-on-exit
  - `src/cli/proot.c` — new CLI option handlers (link2symlink, ashmem-memfd, sysvipc, -L, -H, -p)
  - `src/cli/proot.h` — option table with new options, VERSION set to "5.4.0-pr"
  - `src/extension/extension.h` — new events (SIGSYS_OCC, LINK2SYMLINK_RENAME/UNLINK, STATX_SYSCALL)
  - `src/syscall/seccomp.c` — Android-specific seccomp filter additions (ioctl, memfd_create, statx)
  - Note: 39 additional files also differ but deferred to build-time verification

- [x] **T1.3** Update src/proot/src/GNUmakefile
  - Copied from vendor/termux-proot/src/GNUmakefile
  - `?=` for CC, STRIP, OBJCOPY, OBJDUMP (cross-compilation support)
  - Links libtalloc via `-ltalloc` directly (no pkg-config)
  - 86 object files: new extensions, split fake_id0 (18 files), seccomp, statx, f2fs-bug
  - HAS_POKEDATA_WORKAROUND detection + loader-info.o
  - PROOT_UNBUNDLE_LOADER support
  - Loader uses .exe extension, --rosegment linker flag
  - Also copied vendor/termux-proot fake_id0 split files (18 .c + 17 .h)

- [x] **T1.4** Create build.sh for NDK cross-compilation (builds from src/proot/)
  - Auto-downloads Android NDK r27c if not found (accepts NDK_PATH env var)
  - Creates standalone toolchain per architecture (API 28)
  - Clones and cross-compiles libtalloc as static library per arch
  - Builds proot with NDK clang (statically linked, -ltalloc)
  - Binary verification: file(1), readelf -h, check no NEEDED entries
  - Builds both aarch64 and arm by default (--arch= flag for single arch)
  - Output: build/out/arm64/proot, build/out/arm/proot

- [ ] **T1.5** Test proot binary on Android device
  - Push binary via adb
  - Test: `proot --link2symlink --root-id -r /some/rootfs /bin/sh`
  - Verify SIGSYS handling (no crash on open, stat, chmod)
  - Verify link2symlink (create hard links)
  - Verify --kill-on-exit (orphan cleanup)

## Phase 2 — Standalone proot-distro.sh

- [ ] **T2.1** Fork proot-distro.sh from vendor/termux-proot-distro
  - Copy to `src/scripts/proot-distro.sh`
  - Replace shebang: `#!/system/bin/sh` or `#!${APP_PREFIX}/bin/busybox sh`

- [ ] **T2.2** Replace all template variables
  - `@TERMUX_PREFIX@` → `${APP_PREFIX}`
  - `@TERMUX_HOME@` → `${APP_HOME}`
  - `@TERMUX_APP_PACKAGE@` → `${APP_PACKAGE}`
  - Ensure APP_PREFIX, APP_HOME, APP_PACKAGE are set from environment

- [ ] **T2.3** Remove all Termux-specific code
  - Remove DISTRO_TYPE="termux" code paths in command_install
  - Remove DISTRO_TYPE="termux" code paths in command_login
  - Remove LD_PRELOAD save/restore
  - Remove --termux-home option and bind logic
  - Remove --shared-tmp option and bind logic
  - Remove Termux prefix bind mount
  - Remove Termux data directory bind mounts
  - Remove GNU bash/tar version checks
  - Remove dpkg architecture check
  - Remove Termux paths from detect_cpu_arch
  - Remove Termux-specific help text
  - Remove termux.sh plugin reference

- [ ] **T2.4** Adapt download mechanism
  - Replace `curl -Lo` with `busybox wget -O`
  - Add retry logic around wget (3 retries, 10s timeout)
  - Verify SHA-256 with busybox sha256sum

- [ ] **T2.5** Adapt dependency check
  - Remove `unzip`, `lscpu`, `curl` from required list
  - Verify utilities via busybox applet symlinks
  - Add `proot` check against `${APP_PREFIX}/bin/proot`

- [ ] **T2.6** Adapt CPU detection
  - Remove Termux paths from detect_cpu_arch binary search list
  - Replace lscpu usage with /proc/cpuinfo parsing
  - Ensure busybox `file` output is handled correctly

- [ ] **T2.7** Add self_initialize() function
  - Directory creation
  - Binary verification (proot, busybox)
  - Plugin directory check

- [ ] **T2.8** Update DEFAULT_FAKE_KERNEL_RELEASE
  - Change to `6.17.0-pr`

- [ ] **T2.9** Update DEFAULT_PATH_ENV
  - Remove @TERMUX_PREFIX@/bin and /system/bin references
  - Use `${APP_PREFIX}/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin`

- [ ] **T2.10** Test proot-distro.sh via adb shell
  - Install Alpine Linux
  - Login to Alpine
  - Remove Alpine
  - Test --isolated mode
  - Test backup/restore

## Phase 3 — Busybox Integration

- [ ] **T3.1** Obtain static busybox binary for aarch64
  - Download pre-built or build from source with NDK
  - Verify all required applets present: `busybox --list`

- [ ] **T3.2** Create busybox bootstrap function
  - Copy binary to `${APP_PREFIX}/bin/busybox`
  - `chmod 755 busybox`
  - Create applet symlinks via `busybox --install -s ${APP_PREFIX}/bin/`

- [ ] **T3.3** Verify tar compatibility
  - Test extraction of .tar.xz, .tar.gz, .tar.bz2 rootfs tarballs
  - Document any busybox tar limitations

- [ ] **T3.4** Verify file command compatibility
  - Test `busybox file` against aarch64 and arm ELF binaries
  - Ensure detect_cpu_arch regex handles busybox output

## Phase 4 — Android APK

- [ ] **T4.1** Create Android project structure
  - Package: `id.or.oo.pr`
  - Min SDK 28, Target SDK 35
  - Add INTERNET, FOREGROUND_SERVICE permissions
  - Add MANAGE_EXTERNAL_STORAGE permission (optional)

- [ ] **T4.2** Implement BootstrapService (first-run init)
  - Extract proot from native lib to files/usr/bin/
  - Extract busybox from assets to files/usr/bin/
  - Create applet symlinks
  - Copy proot-distro.sh from assets to files/usr/scripts/
  - Copy plugins from assets to files/usr/etc/proot-distro/
  - Create all data directories

- [ ] **T4.3** Implement MainActivity
  - List available distros (read plugin files)
  - Show install status for each
  - Install button → async install via proot-distro.sh
  - Login button → launch TerminalActivity
  - Remove button → confirm dialog → async remove

- [ ] **T4.4** Implement TerminalActivity
  - Embed terminal emulator View
  - Launch proot-distro.sh login via ProcessBuilder
  - Wire process I/O to terminal
  - Handle terminal resize
  - Handle process exit

- [ ] **T4.5** Implement ProotLauncher
  - Construct environment variables
  - Build command line
  - Execute process and return to caller

- [ ] **T4.6** Bundle proot as native library
  - Place built proot binary in `app/src/main/jniLibs/arm64-v8a/libproot.so`
  - Android extracts to native lib path automatically

- [ ] **T4.7** Bundle assets
  - `assets/bin/busybox-arm64`
  - `assets/scripts/proot-distro.sh`
  - `assets/plugins/alpine.sh`, `debian.sh`, `ubuntu.sh`, `archlinux.sh`, `fedora.sh`

## Phase 5 — Distro Plugins & Testing

- [ ] **T5.1** Port distro plugins
  - Copy alpine.sh, debian.sh, ubuntu.sh, archlinux.sh, fedora.sh
  - Exclude termux.sh
  - Verify TARBALL_URLs are accessible
  - Audit distro_setup() hooks for Termux references

- [ ] **T5.2** Integration test: Alpine
  - Install Alpine via app UI
  - Login and verify shell works
  - Run `apk update && apk add vim`
  - Remove Alpine

- [ ] **T5.3** Integration test: Debian
  - Install Debian via app UI
  - Login and verify shell works
  - Run `apt update && apt install -y vim`
  - Verify locale setup (distro_setup ran)
  - Remove Debian

- [ ] **T5.4** Integration test: Ubuntu
  - Install Ubuntu
  - Login and verify
  - Remove Ubuntu

- [ ] **T5.5** Integration test: backup/restore
  - Install Alpine
  - Make changes (install a package)
  - Backup Alpine
  - Remove Alpine
  - Restore from backup
  - Verify changes preserved

- [ ] **T5.6** Integration test: --isolated mode
  - Install Debian
  - Login with --isolated
  - Verify /sdcard, /system, /vendor are NOT accessible
  - Verify /usr, /bin, /etc are accessible

## Phase 6 — Polish & Documentation

- [ ] **T6.1** Rootfs mirror setup
  - Configure pr.oo.or.id/dl/rootfs/ as fallback mirror
  - Add mirror URL configuration to app settings

- [ ] **T6.2** Error handling
  - Handle download failures gracefully
  - Handle extraction failures (clean up partial rootfs)
  - Handle proot crash (inform user)
  - Handle SELinux ptrace denial (show explanatory message)

- [ ] **T6.3** README and user documentation
  - Write README.md for github.com/oonid/pr
  - Document supported devices and known limitations
  - Document how to add custom distro plugins
  - Document how to build from source

- [ ] **T6.4** CI/CD setup
  - GitHub Actions workflow for building proot binary
  - GitHub Actions workflow for building APK
  - Release automation

- [ ] **T6.5** App signing and release
  - Generate signing key
  - Configure release build type
  - Create first release APK
  - Push to github.com/oonid/pr releases
