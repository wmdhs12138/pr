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
  - Copied 14 distro plugins to `src/scripts/plugins/` (excluded termux.sh, oracle.sh, pardus.sh, void.sh)
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

- [x] **T3.1** Obtain static busybox binary for aarch64
  - Downloaded Alpine busybox-static v1.37.0-r14 (1.1MB, static aarch64)
  - Pinned package with SHA256 verification of both APK and extracted binary
  - Script: download-busybox.sh (reproducible, cached downloads)
  - Output: build/assets/arm64-v8a/busybox (ready for APK bundling)
  - No PT_TLS segment (no alignment fix needed)
  - All required applets verified on-device in T2.10
  - License: GPL-2.0-only

- [x] **T3.2** Obtain static bash binary for aarch64
  - Downloaded robxu9/bash-static v5.2.015 (2.3MB, static aarch64)
  - SHA256 pinned, reproducible download script
  - Script: download-bash.sh (same pattern as download-busybox.sh)
  - Output: build/assets/arm64-v8a/bash (ready for APK bundling)
  - No PT_TLS segment (no alignment fix needed)
  - Verified on-device in T2.10 (associative arrays, mapfile, [[ ]], etc.)
  - License: GPL-3.0-or-later

- [x] **T3.3** Create bootstrap script
  - Created src/scripts/bootstrap.sh (POSIX sh, no bash dependency)
  - Creates directory structure: bin, etc/proot-distro, var/lib/proot-distro, home, tmp, scripts
  - Installs busybox + creates 311 applet symlinks via busybox --list
  - Installs bash (overwrites busybox's bash symlink with real static bash)
  - Installs proot binary
  - Copies proot-distro.sh to bin/proot-distro, replaces @APP_PREFIX@ shebang template
  - Copies distro plugins from plugins/ to etc/proot-distro/
  - Idempotent: skips if .bootstrapped marker file exists
  - Updated test-push.sh to use bootstrap.sh flow with build/assets/ binaries
  - Verified on device: all commands work end-to-end (list, install, login, remove)

- [x] **T3.4** Verify tar compatibility
  - All 14 distro plugins use .tar.xz exclusively (no .tar.gz or .tar.bz2 in production)
  - Tested busybox tar v1.37.0 extraction of all three formats on device:
    - .tar.xz: OK (files, symlinks, subdirs, --strip, proot --link2symlink wrapper)
    - .tar.gz: OK (files, symlinks, subdirs)
    - .tar.bz2: OK (files, symlinks, subdirs)
  - `--strip-components=N` works correctly
  - proot `--link2symlink` tar wrapper works correctly
  - No need to bundle GNU tar as fallback
  - Document any busybox tar limitations

- [x] **T3.5** Replace file command with ELF header parsing
  - Critical finding: Alpine's busybox-static does NOT include `file` applet
  - Replaced detect_cpu_arch() with ELF e_machine field parsing using `od`
  - Reads 2 bytes at offset 18 (e_machine) from ELF header directly
  - Architecture mapping: 0xb7→aarch64, 0x28→arm, 0x3e→x86_64, 0x03→i686, 0xf3→riscv64, 0x08→mips
  - Updated dependency check: removed `file`, added `dd` and `hexdump`
  - More reliable than file(1) — reads binary format directly
  - Verified on device: detect_cpu_arch correctly identifies aarch64 from Alpine rootfs

## Phase 4 — Android APK

- [x] **T4.1** Create Android project structure
  - Package: `id.or.oo.pr`
  - Min SDK 28, Target SDK 35, compileSdk 36
  - Permissions: INTERNET, FOREGROUND_SERVICE, MANAGE_EXTERNAL_STORAGE (optional)
  - Language: Kotlin 2.1 with AGP 8.7.3, Gradle 8.11.1
  - Project lives in android/ subdirectory (separate from native build)
  - Placeholder MainActivity with basic layout
  - AppCompat theme (dark action bar)
  - arm64-v8a only (abiFilters)
  - Debug APK builds successfully (13MB before assets)

- [x] **T4.2** Implement BootstrapService (first-run init)
  - Implemented as App.kt (Application class), runs on every app launch
  - Copies busybox + bash from assets/bin/ to files/usr/bin/
  - Copies proot from nativeLibraryDir (jniLibs/libproot.so) to files/usr/bin/proot
  - Copies bootstrap.sh + proot-distro.sh from assets/scripts/ to files/usr/scripts/
  - Copies 14 distro plugins from assets/plugins/ to files/usr/etc/proot-distro/
  - Executes bootstrap.sh via /system/bin/sh with env vars (APP_PREFIX, PROOT_NO_SECCOMP=1)
  - bootstrap.sh handles: chmod 755, applet symlinks, shebang template replacement
  - Idempotent via SharedPreferences bootstrap_version (increment to re-bootstrap)
  - APK verified: 15MB with all assets + native lib bundled

- [x] **T4.3** Implement MainActivity
  - Compose-based UI with Material3 theme
  - Reads plugin files from files/usr/etc/proot-distro/ to list distros
  - Checks files/usr/var/lib/proot-distro/installed-rootfs/ for install status
  - Install button → async install via proot-distro.sh with live output view
  - Login button → launches TerminalActivity with distro name
  - Remove button → async remove via proot-distro.sh with live output view
  - Parses DISTRO_NAME from plugin .sh files for display names
  - Uses coroutines for background I/O (Dispatchers.IO)

- [x] **T4.4** Implement TerminalActivity
  - Uses ConnectBot termlib Terminal() composable for terminal rendering
  - Creates TerminalEmulator via TerminalEmulatorFactory.create()
  - PTY reader thread reads master fd → feeds to emulator.writeInput()
  - onKeyboardInput callback writes to PTY master fd
  - Dark theme: background #1a1a2e, foreground white, font 12sp
  - Handles back press → cleanup and finish
  - Lifecycle: cleanup on destroy

- [x] **T4.5** Implement ProotLauncher
  - PtyNative JNI shim: fork/exec with PTY via /dev/ptmx
  - ptyjni.c (~180 lines C): forkPty, read, write, resize, close, getPid, waitPid
  - ProotLauncher.startSession(): forks bash + proot-distro.sh login with PTY
  - ProotLauncher.runCommand(): forks bash -c for install/remove commands
  - Environment: APP_PREFIX, APP_HOME, APP_PACKAGE, PROOT_NO_SECCOMP=1, TERM=xterm-256color
  - Session class wraps master fd with read/write/resize/close operations

- [x] **T4.6** Bundle proot as native library
  - Placed at `app/src/main/jniLibs/arm64-v8a/libproot.so` (2.5MB)
  - Android extracts to nativeLibraryDir automatically
  - `extractNativeLibs=true` + `useLegacyPackaging=true` for uncompressed extraction
  - BootstrapService copies from nativeLibraryDir to files/usr/bin/proot

- [x] **T4.7** Bundle assets
  - `assets/bin/busybox` (1.1MB, from build/assets/arm64-v8a/busybox)
  - `assets/bin/bash` (2.3MB, from build/assets/arm64-v8a/bash)
  - `assets/scripts/proot-distro.sh` (with @APP_PREFIX@ template)
  - `assets/scripts/bootstrap.sh` (POSIX sh setup script)
  - `assets/plugins/*.sh` (14 distro plugins)
  - Total APK: 15MB (debug, uncompressed)

## Phase 5 — Distro Plugins & Testing

- [x] **T5.1** Port distro plugins
  - 14 distro plugins bundled (removed oracle.sh, pardus.sh, void.sh — stale tarball versions)
  - Excluded termux.sh (not present in our plugin set)
  - All TARBALL_URLs verified accessible (HTTP 200, easycli.sh host)
  - Zero Termux references in any plugin — all distro_setup() hooks use standard Linux commands only
  - 7 plugins have distro_setup(): archlinux, artix, debian, fedora, manjaro, opensuse, trisquel, ubuntu, void
  - Remaining: adelie, almalinux, alpine, archlinux, artix, chimera, debian, deepin, fedora, manjaro, opensuse, rockylinux, trisquel, ubuntu

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

## Phase 6 — Replace proot-distro.sh with Rust Binary (pr-cli)

Viability confirmed (see `docs/bash-to-rust.md`): a statically-linked Rust binary
in nativeLibraryDir can be exec'd from the app process (`untrusted_app` SELinux).
Rust can fork+exec subcommands (busybox, /system/bin/sh, proot, bash), perform file
I/O, read env vars, and parse plugin configs. 639KB for the test binary.

**Why Rust instead of mksh port:** The mksh port requires replacing all associative
arrays with `eval`-based flat-variable hacks across 3000+ lines of shell — fragile,
hard to test, hard to maintain. Rust gives us type safety, testability, and avoids
the entire shell compatibility problem. Additionally, Rust can exec bash as a
subprocess (exit=0) while Java ProcessBuilder cannot (SIGSYS 159) — Rust acts as
a trusted intermediary.

**Binary:** `src/pr-cli/` — statically linked for `aarch64-linux-android`, bundled as
`jniLibs/arm64-v8a/libpr-cli.so`, symlinked to `files/usr/bin/pr-cli`.
Cross-compiled via NDK 27 clang + `cargo build --target aarch64-linux-android`.

### T6.1 — Rust project scaffolding

- [ ] Create `src/pr-cli/` Cargo project with cross-compilation config
- [ ] `.cargo/config.toml` with NDK 27 linker and static link flags
- [ ] `Cargo.toml` with dependencies: `clap` (CLI), `sha2` (SHA256), `libc`
- [ ] `build-pr-cli.sh` script: build, strip, copy to jniLibs
- [ ] Add `libpr-cli.so` symlink to `App.kt` ensureNativeLibSymlinks()
- [ ] Bump `BOOTSTRAP_VERSION`

### T6.2 — Plugin config parser

- [ ] Parse key=value format from `.sh` plugin files
- [ ] Handle both formats: `TARBALL_URL_aarch64="..."` (flat) and `TARBALL_URL['aarch64']="..."` (legacy)
- [ ] Extract `DISTRO_NAME`, `DISTRO_COMMENT`, `TARBALL_URL_<arch>`, `TARBALL_SHA256_<arch>`
- [ ] Handle `distro_setup()` detection (present/absent in plugin)
- [ ] Unit tests with all 14 real plugins as test fixtures

### T6.3 — CLI interface and `command_list`

- [ ] Subcommands: `install`, `login`, `remove`, `list`, `backup`, `restore`, `rename`, `reset`, `copy`, `clear-cache`
- [ ] `pr-cli list` — iterate plugins, display distro name, comment, supported architectures
- [ ] Colored output (match proot-distro.sh format for familiarity)
- [ ] `--help` and `--version` flags

### T6.4 — `command_install`

- [ ] Argument parsing: `--override-alias`, `--override-tarbll-url`, `--override-tarball-sha256`
- [ ] Download tarball via busybox wget subprocess (with retry, 3 attempts)
- [ ] SHA256 verification via busybox sha256sum subprocess
- [ ] Extract tarball via busybox tar subprocess (`--link2symlink` wrapper if needed)
- [ ] Write config files: `/etc/passwd`, `/etc/group`, `/etc/resolv.conf`, `/etc/environment`
- [ ] Generate fake `/proc` data (port `setup_fake_sysdata()` from shell)
- [ ] Handle `--override-alias` (copy plugin, rewrite DISTRO_NAME)
- [ ] Call `distro_setup()` via proot if plugin has one

### T6.5 — `command_login`

- [ ] Argument parsing: `--user`, `--isolated`, `--shared-tmp`, `--no-link2symlink`, `--no-sysvipc`, `--custom-bind`, `--cpu-emulator`
- [ ] Build proot command line: bind mounts, env vars, kernel version fake, symlinks
- [ ] Detect bind-mountable system dirs via `stat -c '%a'` (port the `case "${mode:2}"` logic)
- [ ] Handle CPU emulation (qemu args for cross-arch)
- [ ] Write `/etc/environment` with current Android env vars
- [ ] `exec` proot (replace Rust process — no subshell)

### T6.6 — `command_remove`, `command_reset`, `command_clear-cache`

- [ ] `remove` — delete rootfs directory, clean up symlinks
- [ ] `reset` — remove + re-install (call install flow)
- [ ] `clear-cache` — delete download cache entries

### T6.7 — `command_backup`, `command_restore`, `command_rename`, `command_copy`

- [ ] `backup` — tar the rootfs into a backup archive
- [ ] `restore` — extract backup archive to rootfs
- [ ] `rename` — rename distro alias, update plugin symlink
- [ ] `copy` — copy rootfs from one distro to another

### T6.8 — APK integration and end-to-end testing

- [ ] Update `MainActivity.kt` ProcessBuilder to invoke `pr-cli install alpine` instead of `/system/bin/sh proot-distro.sh`
- [ ] Update `ProotLauncher.kt` to invoke `pr-cli login alpine`
- [ ] Integration test: install Alpine from app UI
- [ ] Integration test: login to Alpine from app UI
- [ ] Integration test: install Debian, run `apt update`
- [ ] Integration test: backup/restore Alpine
- [ ] Verify APK size delta (pr-cli binary vs shell script)

### T6.9 — Cleanup

- [ ] Remove `proot-distro.sh` from assets (replaced by pr-cli)
- [ ] Remove bash binary from jniLibs (no longer needed as script interpreter)
- [ ] Keep `bootstrap.sh` (still needed for initial directory setup + busybox applet symlinks)
- [ ] Update `docs/bash-to-rust.md` with final results

## Phase 7 — Polish & Documentation

- [ ] **T7.1** Rootfs mirror setup
  - Configure pr.oo.or.id/dl/rootfs/ as fallback mirror
  - Add mirror URL configuration to app settings

- [ ] **T7.2** Error handling
  - Handle download failures gracefully
  - Handle extraction failures (clean up partial rootfs)
  - Handle proot crash (inform user)
  - Handle SELinux ptrace denial (show explanatory message)

- [ ] **T7.3** README and user documentation
  - Write README.md for github.com/oonid/pr
  - Include project name explanation: pr = PRoot = ptrace-based root (see docs/name.md)
  - Document supported devices and known limitations
  - Document how to add custom distro plugins
  - Document how to build from source

- [ ] **T7.4** CI/CD setup
  - GitHub Actions workflow for building proot binary
  - GitHub Actions workflow for building APK
  - Release automation

- [ ] **T7.5** App signing and release
  - Generate signing key
  - Configure release build type
  - Create first release APK
  - Push to github.com/oonid/pr releases

## Phase 8 — mksh Port of proot-distro.sh (Optional Alternative)

This is the alternative to Phase 6's Rust approach. Only needed if we decide
against Rust. The mksh port is fragile (eval-heavy associative array simulation)
and harder to maintain, but avoids adding a Rust toolchain dependency.

Android's `untrusted_app` SELinux domain blocks `execve()` of binaries in
app-writable directories. The app process can only exec `/system/bin/sh` (mksh
R59 2020/10/31). Bash in nativeLibraryDir works from `run-as` (which uses
`runas_app` context) but is killed with SIGSYS (exit 159) when exec'd from the
app process via ProcessBuilder.

**Goal:** Make proot-distro.sh fully compatible with mksh R59 so it can be
invoked as `/system/bin/sh proot-distro.sh ...` from the app process.

**Key mksh R59 limitations discovered:**
- No `typeset -A` / `declare -A` (associative arrays) — silently corrupts data
- No `mapfile` / `readarray`
- No process substitution `< <(...)`
- No `${!var}` indirect expansion
- No `[[ =~ ]]` regex match
- No `local -a` (use plain `local`)
- Here-docs with command substitution need writable `TMPDIR`
- Plugin array keys must NOT be quoted: `x[aarch64]` OK, `x['aarch64']` NOT OK
- mksh parses the ENTIRE script before executing — bash-isms in `command_login()`
  break `command_install()` too

- [ ] **T8.1** Replace associative arrays with flat variables and eval helpers
- [ ] **T8.2** Fix remaining mksh incompatibilities in proot-distro.sh
- [ ] **T8.3** Update plugin loading for flat variables
- [ ] **T8.4** Test proot-distro.sh under mksh R59 on device
- [ ] **T8.5** Test install from app UI via ProcessBuilder with `/system/bin/sh`
- [ ] **T8.6** Test login from app UI
