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

- [x] **T1.5** Test proot binary on Android device (unrooted, Android 16 / SDK 36, Samsung)
  - TLS alignment fix required: Bionic rejects TLS segment with < 64-byte alignment
    - Fixed via post-build Python script that patches PT_TLS p_align in ELF header
    - Integrated into build.sh as fix_tls_alignment() step
  - PROOT_NO_SECCOMP=1 required on this device (seccomp policy blocks ptrace-execve)
    - APK must set this environment variable by default
  - Tested with Alpine 3.21.3 minirootfs on arm64:
    - `proot --version` → v5.4.0-pr ✓
    - Fake root: `id` → uid=0(root) gid=0(root) ✓
    - Alpine rootfs: `ls /` shows Alpine dirs, `cat /etc/os-release` correct ✓
    - link2symlink: hard link created and readable ✓
    - --kill-on-exit: background process killed on main exit ✓
    - Interactive shell: works (PATH must be set by launcher) ✓
  - Device context: u:r:shell:s0 (adb shell), non-rooted
  - New source file: src/proot/src/tls-align.c (TLS dummy variable for alignment)

## Phase 2 — Standalone proot-distro.sh

- [x] **T2.1** Fork proot-distro.sh from vendor/termux-proot-distro
  - Copied to `src/scripts/proot-distro.sh` (3172 lines)
  - Shebang: `#!@APP_PREFIX@/bin/bash` (template, replaced by APK BootstrapService)
  - Decision: bundle static bash binary (~2MB) — proot-distro.sh requires bash
    features (declare -A associative arrays, [[ ]] tests, bash string manipulation)
    that busybox ash does not support
  - Copied 17 distro plugins to `src/scripts/plugins/` (excluded termux.sh)
  - Plugins: alpine, almalinux, archlinux, artix, adelie, chimera, debian,
    deepin, fedora, manjaro, opensuse, oracle, pardus, rockylinux, trisquel,
    ubuntu, void
  - @TERMUX_PREFIX@, @TERMUX_HOME@, @TERMUX_APP_PACKAGE@ still present in file;
    will be replaced in T2.2

- [x] **T2.2** Replace all template variables
  - `@TERMUX_PREFIX@` → `${APP_PREFIX}` (21 occurrences)
  - `@TERMUX_HOME@` → `${APP_HOME}` (5 occurrences)
  - `@TERMUX_APP_PACKAGE@` → `${APP_PACKAGE}` (4 occurrences)
  - Added runtime variable definitions at top of script with defaults:
    - `APP_PREFIX="${APP_PREFIX:-/data/data/id.or.oo.pr/files/usr}"`
    - `APP_HOME="${APP_HOME:-/data/data/id.or.oo.pr/files/home}"`
    - `APP_PACKAGE="${APP_PACKAGE:-id.or.oo.pr}"`
  - Removed `TERMUX_LDPRELOAD` save (no longer needed without Termux)
  - Updated comments referencing Termux prefix to say "App prefix"
  - Verified: `grep -c '@TERMUX_' src/scripts/proot-distro.sh` = 0
  - Verified: `bash -n src/scripts/proot-distro.sh` passes with no errors
  - Verified: plugins have zero @TERMUX_*@ references

- [x] **T2.3** Remove all Termux-specific code
  - Removed DISTRO_TYPE="termux" code paths in command_install (entire zip/SYMLINKS block)
  - Removed DISTRO_TYPE="termux" code paths in command_login (termux env/shell setup)
  - Removed DISTRO_TYPE="termux" code paths in run_proot_cmd (termux proot invocation)
  - Removed LD_PRELOAD save/restore (TERMUX_LDPRELOAD + all unset/restore patterns)
  - Removed --termux-home option parsing and bind logic
  - Removed --shared-tmp option parsing and bind logic
  - Removed Termux prefix bind mount guards (always bind now)
  - Removed GNU bash/tar version checks (3 occurrences)
  - Removed dpkg architecture check at entry point
  - Removed lscpu 32-bit check — replaced with /proc/cpuinfo parsing
  - Removed curl, lscpu, unzip, xz from dependency check
  - Removed Termux path from detect_cpu_arch search list
  - Removed Termux-specific help text (--termux-home, --shared-tmp, termux distro notes)
  - Fixed GECOS field in passwd: "Termux" → "proot-distro"
  - Fixed fake /proc/version: "proot@termux" → "proot@pr"
  - Script reduced from 3176 to 2957 lines (219 lines removed)
  - All `!= "termux"` guards simplified (always true, removed condition)
  - Syntax verified: bash -n passes

- [x] **T2.4** Adapt download mechanism
  - Created `download_file()` helper function with retry + fallback
  - Tries curl first (if available), falls back to wget (busybox)
  - Retry: 3 attempts with exponential backoff (5s, 10s, 20s, max 60s)
  - curl: --disable --fail --location --connect-timeout 15 --max-time 600
  - wget: -T 30 -q (busybox compatible)
  - Validates downloaded file exists and is non-empty
  - Removed .tmp rename pattern (download_file writes directly, cleans up on failure)
  - SHA-256 verification unchanged (sha256sum works in busybox)

- [x] **T2.5** Adapt dependency check
  - Removed `bzip2` (not used anywhere in script)
  - Removed `curl` (already done in T2.3, now optional via download_file fallback)
  - Added `realpath` (8 uses in command_login for path resolution)
  - Added `stat` (9 uses for directory permission checks)
  - Added `sha256sum` (used for rootfs integrity verification)
  - Added `wget` (needed as download_file fallback when curl unavailable)
  - Final list: awk, basename, cat, chmod, cp, cut, du, file, find, grep,
    gzip, head, id, mkdir, proot, realpath, rm, sed, sha256sum, stat, tar,
    wget, xargs (23 utilities, all busybox applets or our own binaries)
  - Add `proot` check against `${APP_PREFIX}/bin/proot`

 - [x] **T2.6** Adapt CPU detection
  - Termux paths already removed from detect_cpu_arch in T2.3
  - lscpu already replaced with /proc/cpuinfo in T2.3
  - detect_cpu_arch: replaced fragile `cut` pipeline with `grep -oE` regex
    - Old: `file -L | cut -d':' -f2- | cut -d',' -f2 | cut -d' ' -f2-`
    - New: `file -L | grep -oE '(ARM aarch64|ARM|UCB RISC-V|Intel 80386|x86-64|MIPS)'`
    - Handles both GNU file and busybox file output formats
    - Added MIPS architecture mapping
  - Verified: all arch patterns match for both GNU and busybox file output

- [x] **T2.7** Add self_initialize() function
  - Called once at script startup, after dependency check
  - Verifies proot binary exists in PATH (clear error message with expected location)
  - Verifies DISTRO_PLUGINS_DIR exists
  - Verifies at least one plugin .sh file is present
  - Creates RUNTIME_DIR, INSTALLED_ROOTFS_DIR, DOWNLOAD_CACHE_DIR if missing
  - Removed redundant mkdir in command_install (now handled upfront)
  - Exits with clear error messages if environment is broken

- [x] **T2.8** Update DEFAULT_FAKE_KERNEL_RELEASE
  - Changed from `6.17.0-PRoot-Distro` to `6.17.0-pr`
  - Matches proot binary version string (5.4.0-pr)

- [x] **T2.9** Update DEFAULT_PATH_ENV
  - Removed `/system/bin:/system/xbin` (Android system paths should not be in distro PATH)
  - Kept `${APP_PREFIX}/bin` (our bundled busybox/bash binaries)
  - Final: `/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/local/games:/usr/games:${APP_PREFIX}/bin`

- [x] **T2.10** Test proot-distro.sh via adb shell
  - Fixed busybox compatibility: grep -P→-E, stat --format→-c, realpath -m/-q, paste→bash arrays, GNU tar options
  - proot-distro list: lists 17 distributions
  - proot-distro install alpine: downloads + extracts Alpine 3.23 rootfs (with cached tarball)
  - proot-distro login alpine: logs in as root, uname shows 6.17.0-pr
  - proot-distro remove alpine: cleans up rootfs
  - NOT tested: --isolated mode, backup/restore (deferred to Phase 5)
  - See docs/phase2.md for full test results and follow-up items

## Phase 3 — Busybox & Bash Integration

- [ ] **T3.1** Obtain static busybox binary for aarch64
  - Download pre-built or build from source with NDK
  - Verify all required applets present: `busybox --list`

- [ ] **T3.2** Obtain static bash binary for aarch64
  - proot-distro.sh requires bash (associative arrays, [[ ]], etc.)
  - Download pre-built static bash or cross-compile with NDK
  - ~2MB binary

- [ ] **T3.3** Create bootstrap function
  - Copy busybox to `${APP_PREFIX}/bin/busybox`
  - Copy bash to `${APP_PREFIX}/bin/bash`
  - `chmod 755` both
  - Create applet symlinks via `busybox --install -s ${APP_PREFIX}/bin/`

- [ ] **T3.4** Verify tar compatibility
  - Test extraction of .tar.xz, .tar.gz, .tar.bz2 rootfs tarballs
  - Document any busybox tar limitations

- [ ] **T3.5** Verify file command compatibility
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
  - Extract bash from assets to files/usr/bin/
  - Create applet symlinks via busybox --install
  - Copy proot-distro.sh from assets to files/usr/bin/
  - Replace @APP_PREFIX@ in proot-distro.sh shebang with actual path
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
  - `assets/bin/bash-arm64`
  - `assets/bin/proot-distro.sh` (with @APP_PREFIX@ template)
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
