# Design: Initial Implementation of Standalone proot-distro for Android

## Architecture Overview

```
┌───────────────────────────────────────────────────────────────┐
│                    Android APK (id.or.oo.pr)                │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ MainActivity │  │ Terminal     │  │ DistroManager UI     │ │
│  │ (launcher)   │  │ Activity     │  │ (install/remove/list)│ │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘ │
│         │                 │                      │             │
│         └────────┬────────┴──────────────────────┘             │
│                  │                                             │
│         ┌───────▼────────┐                                    │
│         │ ProotLauncher  │  (JNI / Runtime.exec)              │
│         │ (native bridge)│                                    │
│         └───────┬────────┘                                    │
│                 │                                              │
├─────────────────┼──────────────────────────────────────────────┤
│                 ▼  App-Private Data                            │
│  /data/data/id.or.oo.pr/files/usr/                         │
│  ├── bin/                                                     │
│  │   ├── proot           (patched, static aarch64 binary)     │
│  │   ├── busybox         (static, provides all utils)         │
│  │   ├── sh -> busybox                                        │
│  │   └── (applet symlinks)                                    │
│  ├── etc/proot-distro/                                        │
│  │   ├── debian.sh                                            │
│  │   ├── alpine.sh                                            │
│  │   └── ...                                                  │
│  ├── scripts/                                                 │
│  │   └── proot-distro.sh  (standalone port)                   │
│  ├── home/                                                    │
│  ├── tmp/                                                     │
│  └── var/lib/proot-distro/                                    │
│      ├── dlcache/                                             │
│      └── installed-rootfs/                                    │
│          └── alpine/                                          │
│              ├── .l2s/                                        │
│              ├── bin/, etc/, usr/, ...                        │
│              └── proc/, sys/                                  │
└───────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Patched proot Binary

**Source:** `vendor/proot/` with cherry-picked patches from `vendor/termux-proot/`.

**Patches applied (by priority):**

P0 — Must-have for Android:
- `src/tracee/seccomp.c` + `seccomp.h` — SIGSYS handler translating 40+ blocked syscalls
- `src/arch.h` — ARM64 definitions, POKEDATA workaround flag, loader addresses
- `src/tracee/mem.c` — ARM64 POKEDATA workaround assembly stub
- `src/loader/assembly-arm64.h` — ARM64 loader assembly
- `src/extension/link2symlink/` — Hard link emulation via symlinks (SELinux)

P1 — Strongly recommended:
- `src/extension/ashmem_memfd/` — memfd_create fallback via /dev/ashmem
- `src/tracee/statx.c` + `statx.h` — statx() syscall handling
- `--kill-on-exit` option — tracee lifecycle management

P2 — Quality of life:
- `src/extension/sysvipc/` — System V IPC emulation
- `src/path/f2fs-bug.c` + `f2fs-bug.h` — f2fs case-sensitivity workaround
- `src/extension/fix_symlink_size/` — symlink st_size correction
- `src/extension/hidden_files/` — hide .proot.* files
- `src/extension/port_switch/` — protected port remapping
- `src/extension/mountinfo/` — mount information

P3 — Compatibility:
- `is_aarch32` support in tracee.h
- Enhanced syscall chaining fields
- Seccomp filter additions for Android

**Modified existing files:**
- `src/tracee/tracee.h` — new struct fields (killall_on_exit, seccomp state, aarch32, chain enhancements)
- `src/tracee/event.c` — kill-on-exit logic, seccomp event dispatch
- `src/cli/proot.c` — new CLI option handlers
- `src/cli/proot.h` — option table updates
- `src/extension/extension.h` — new extension events (SIGSYS_OCC, STATX_SYSCALL, etc.)
- `src/syscall/seccomp.c` — Android-specific filter additions
- `src/GNUmakefile` — new objects, conditional CC/STRIP/OBJCOPY assignments

**Build:**
- Android NDK standalone toolchain (aarch64, API 28)
- Static link against libtalloc (cross-compiled)
- Output: single `proot` binary (~1-2MB)

### 2. Standalone proot-distro.sh

**Source:** Forked from `vendor/termux-proot-distro/proot-distro.sh` (3172 lines).

**Template replacements:**

```
@TERMUX_PREFIX@      → ${APP_PREFIX}   (resolved to app's files/usr)
@TERMUX_HOME@        → ${APP_HOME}     (resolved to app's files/home)
@TERMUX_APP_PACKAGE@ → id.or.oo.pr
```

**Removed:**

1. `DISTRO_TYPE="termux"` entire code path (ZIP bootstrap, SYMLINKS.txt, termux login)
2. `LD_PRELOAD` save/restore
3. `--termux-home` option and bind mount
4. `--shared-tmp` option referencing Termux tmp
5. Termux prefix bind mount
6. Termux data directory bind mounts (`/data/data/com.termux/...`)
7. GNU bash and GNU tar version checks (busybox is sufficient)
8. `dpkg` architecture check
9. Termux-specific paths in `detect_cpu_arch`
10. Termux-specific help text
11. `termux.sh` plugin (only used for Termux bootstrap)

**Adapted:**

1. Dependency check — remove `unzip`, `lscpu`; rely on busybox applets
2. `detect_cpu_arch` — remove Termux paths, use `/proc/cpuinfo` for arch detection as fallback
3. 32-bit CPU detection — parse `/proc/cpuinfo` instead of `lscpu`
4. QEMU path — `${APP_PREFIX}/bin/qemu-*` (downloadable, not bundled in v1)
5. Non-isolated bind mounts — use app's own data dir instead of Termux's
6. `run_proot_cmd` — use `${APP_PREFIX}/bin/proot`
7. `DEFAULT_PATH_ENV` — `${APP_PREFIX}/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin`
8. `setup_fake_sysdata` — kernel string `6.17.0-PRoot-Distro` (no Termux reference)
9. `DEFAULT_FAKE_KERNEL_RELEASE` — `6.17.0-pr` (project branded)

**Added:**

1. `bootstrap_busybox()` — extract busybox from assets, create applet symlinks in `${APP_PREFIX}/bin/`
2. `self_initialize()` — create full directory structure on first launch
3. Environment variable contract with Android app:
   - `APP_PREFIX` — base directory for all app files
   - `APP_HOME` — user home directory
   - `APP_PACKAGE` — Android package name

**Kept unchanged (Android-generic):**

- Fake /proc data generation (loadavg, stat, uptime, version, vmstat, cap_last_cap, inotify)
- Fake /sys/fs/selinux masking
- /dev/urandom → /dev/random bind
- Android UID/GID registration in guest /etc/passwd, /etc/group
- /apex, /system, /vendor, /odm, /product bind mounts (non-isolated)
- /linkerconfig binds
- /storage, /sdcard bind mounts
- Android environment variables forwarding
- Isolated/non-isolated execution modes
- All distro management commands (install, login, remove, list, backup, restore, rename, reset, copy)

### 3. Android APK

**Package:** `id.or.oo.pr`
**Min SDK:** 28 (Android 9)
**Target SDK:** 35

**Components:**

| Component | Responsibility |
|---|---|
| `MainActivity` | App entry, show installed distros, install button |
| `TerminalActivity` | Terminal emulator running proot session |
| `DistroManager` | Logic layer: install/remove/backup/restore distros |
| `ProotLauncher` | JNI/native bridge to exec proot with correct arguments |
| `BootstrapService` | First-run: extract busybox, create symlinks, init dirs |

**Initialization flow (first launch):**

```
App starts → BootstrapService
  1. mkdir -p ${files}/usr/bin
  2. mkdir -p ${files}/usr/etc/proot-distro
  3. mkdir -p ${files}/usr/var/lib/proot-distro/{dlcache,installed-rootfs}
  4. mkdir -p ${files}/home ${files}/tmp
  5. Copy busybox from assets → ${files}/usr/bin/busybox
  6. chmod 755 busybox
  7. Create applet symlinks: sh, bash, tar, curl, sed, awk, grep, ...
  8. Copy proot from jniLibs → ${files}/usr/bin/proot
  9. chmod 755 proot
  10. Copy plugins from assets → ${files}/usr/etc/proot-distro/
  11. Copy proot-distro.sh from assets → ${files}/usr/scripts/
  12. chmod 755 proot-distro.sh
  13. Mark initialized (SharedPreferences)
```

**Terminal emulator:** Embed `jackpal.androidterm` or use Termux's terminal
view library (Apache 2.0) as a `View` inside `TerminalActivity`.

**Proot session launch flow:**

```
TerminalActivity
  → ProotLauncher.login(distroName, user, isolated)
    → Construct environment: APP_PREFIX, APP_HOME, APP_PACKAGE
    → Runtime.exec("/system/bin/sh", "-c", "${APP_PREFIX}/scripts/proot-distro.sh login ${distroName}")
    → Wire process I/O to terminal emulator View
```

### 4. Distro Plugins

Copied from `vendor/termux-proot-distro/distro-plugins/` with modifications:

| Plugin | Modification |
|---|---|
| `alpine.sh` | None needed (pure metadata) |
| `debian.sh` | None needed (distro_setup uses generic commands) |
| `ubuntu.sh` | Audit Mozilla PPA setup — may need network fix |
| `archlinux.sh` | None needed |
| `fedora.sh` | None needed |
| `termux.sh` | **Excluded** (Termux-specific) |
| Others | Audit and include as-is |

**Plugin format** (unchanged from upstream):

```bash
DISTRO_NAME="Alpine Linux"
DISTRO_COMMENT="Regular release v3.23.3."
TARBALL_URL['aarch64']="https://easycli.sh/proot-distro/alpine-aarch64-pd-v4.37.0.tar.xz"
TARBALL_SHA256['aarch64']="..."
```

**Rootfs hosting:** Continue using `easycli.sh` tarballs. Add fallback mirror
at `pr.oo.or.id/dl/rootfs/` for reliability.

## Data Flow

### Install Distribution

```
User taps "Install Alpine"
  → DistroManager.install("alpine")
    → exec("proot-distro.sh install alpine")
      → Source alpine.sh plugin
      → Download tarball via busybox wget
      → Verify SHA-256
      → Extract with: proot --link2symlink tar xf ...
      → Post-install: /etc/resolv.conf, /etc/hosts, /etc/passwd, fake /proc, fake /sys
      → (Optional) distro_setup() hook
    → Return success/failure
  → UI updates: Alpine now shows as "installed"
```

### Login to Distribution

```
User taps "Login" on Alpine
  → TerminalActivity.launch("alpine")
    → ProotLauncher.login("alpine", "root", false)
      → exec("proot-distro.sh login alpine")
        → Detect CPU arch of installed rootfs
        → Construct proot command line with binds
        → exec proot with constructed args
      → Wire I/O to terminal View
    → User interacts with Alpine shell
```

## Security Considerations

1. **ptrace permission**: Android allows ptrace within the same UID by default.
   Some OEM SELinux policies may restrict it — document affected devices.
2. **No real root**: proot's `--root-id` fakes root identity; actual UID/GID
   remain the app's unprivileged UID.
3. **No privilege escalation**: No suid binaries, no real mount, no kernel
   module loading possible through proot.
4. **Storage access**: `MANAGE_EXTERNAL_STORAGE` permission needed for
   `/sdcard` bind mount in non-isolated mode.
5. **Network**: Guest has same network access as host app (no isolation).
