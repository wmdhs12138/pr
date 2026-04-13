# Spec: Standalone proot-distro Script

## Capability

A Bash script that manages Linux distribution installations via proot on
Android, fully independent of Termux. Forked from termux-proot-distro v4.38.0
with all Termux dependencies removed.

## Requirements

### REQ-DISTRO-001: Template Variable Resolution

Replace all `@TERMUX_@` placeholders:

| Template | Replacement | Example Value |
|---|---|---|
| `@TERMUX_PREFIX@` | `${APP_PREFIX}` | `/data/data/id.or.oo.pr/files/usr` |
| `@TERMUX_HOME@` | `${APP_HOME}` | `/data/data/id.or.oo.pr/files/home` |
| `@TERMUX_APP_PACKAGE@` | `${APP_PACKAGE}` | `id.or.oo.pr` |

These are resolved at runtime from environment variables set by the Android app.

### REQ-DISTRO-002: Busybox Dependency

All runtime utilities are provided by a static busybox at `${APP_PREFIX}/bin/busybox`:

Required applets: `awk`, `basename`, `bzip2`, `cat`, `chmod`, `cp`, `cut`,
`du`, `find`, `grep`, `gzip`, `head`, `id`, `mkdir`, `rm`, `sed`, `tar`,
`xargs`, `xz`, `sha256sum`, `file`, `wget`, `mknod`, `chown`, `env`, `false`,
`ln`, `ls`, `mv`, `printf`, `pwd`, `readlink`, `realpath`, `stat`, `touch`,
`tr`, `true`, `uname`, `wc`, `which`

Not required (removed from dependency check): `unzip`, `lscpu`, `curl` (replaced
by busybox `wget`).

### REQ-DISTRO-003: Termux Code Removal

The following must be completely removed:

1. `DISTRO_TYPE="termux"` code path in `command_install` and `command_login`
2. `LD_PRELOAD` save/restore (line 34, line 2198 of original)
3. `--termux-home` option and all associated bind mount logic
4. `--shared-tmp` option referencing Termux tmp
5. Termux prefix bind mount (`--bind=@TERMUX_PREFIX@`)
6. Termux data directory bind mounts (`/data/data/com.termux/...`)
7. GNU bash and GNU tar version checks
8. `dpkg` architecture check
9. Termux paths in `detect_cpu_arch` (`/data/data/com.termux/files/usr/bin/bash`)
10. Termux-specific help text
11. `termux.sh` distro plugin (excluded from bundled plugins)

### REQ-DISTRO-004: Adapted Behaviors

1. **Dependency check**: Only verify busybox applets exist, not specific GNU
   tool versions
2. **CPU arch detection**: Use ELF header reading via `dd` + `file` (busybox),
   with `/proc/cpuinfo` as fallback
3. **32-bit CPU detection**: Parse `/proc/cpuinfo` for `aes` or `asimd` flags
   instead of `lscpu`
4. **Download**: Use `busybox wget` instead of `curl`
5. **QEMU path**: `${APP_PREFIX}/bin/qemu-<arch>` (bundled separately, not in
   v1)
6. **`DEFAULT_FAKE_KERNEL_RELEASE`**: `6.17.0-pr`
7. **`DEFAULT_PATH_ENV`**: `${APP_PREFIX}/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin`
8. **`run_proot_cmd`**: Use `${APP_PREFIX}/bin/proot`

### REQ-DISTRO-005: Self-Initialization

New `self_initialize()` function called at script startup if not yet initialized:

1. Create directory structure:
   - `${APP_PREFIX}/bin/`
   - `${APP_PREFIX}/etc/proot-distro/`
   - `${APP_PREFIX}/var/lib/proot-distro/dlcache/`
   - `${APP_PREFIX}/var/lib/proot-distro/installed-rootfs/`
   - `${APP_HOME}/`
   - `${APP_PREFIX}/tmp/`
2. Verify busybox exists at `${APP_PREFIX}/bin/busybox`
3. Verify proot exists at `${APP_PREFIX}/bin/proot`
4. Copy plugins from `${APP_PREFIX}/etc/proot-distro/` if empty

### REQ-DISTRO-006: Supported Commands

All commands from upstream proot-distro must work:

| Command | Aliases | Description |
|---|---|---|
| `install` | `add`, `i`, `in`, `ins` | Install a distribution |
| `login` | `sh` | Start a shell session |
| `remove` | `rm` | Uninstall distribution |
| `list` | `li`, `ls` | List available distributions |
| `backup` | `bak`, `bkp` | Backup distribution |
| `restore` | — | Restore from backup |
| `rename` | `mv` | Rename distribution |
| `reset` | — | Remove and reinstall |
| `copy` | `cp` | Copy files to/from distribution |
| `clear-cache` | `clear`, `cl` | Clear download cache |
| `help` | — | Show help |

### REQ-DISTRO-007: Login Options

Supported login options (Termux-specific ones removed):

| Option | Status |
|---|---|
| `--user` | Kept |
| `--fix-low-ports` | Kept |
| `--isolated` | Kept |
| `--bind` | Kept |
| `--no-link2symlink` | Kept |
| `--no-sysvipc` | Kept |
| `--no-kill-on-exit` | Kept |
| `--no-arch-warning` | Kept |
| `--kernel` | Kept |
| `--hostname` | Kept |
| `--work-dir` | Kept |
| `--env` | Kept |
| `--termux-home` | **Removed** |
| `--shared-tmp` | **Removed** (or adapted to use app tmp) |

### REQ-DISTRO-008: Android-Specific Preserved Behaviors

These must remain functional:

- Fake /proc data: loadavg, stat, uptime, version, vmstat, cap_last_cap, inotify
- Fake /sys/fs/selinux (empty directory bind)
- /dev/urandom → /dev/random bind
- Android UID/GID registration in guest /etc/passwd, /etc/group
- /apex, /system, /vendor, /odm, /product bind mounts (non-isolated)
- /linkerconfig binds
- /storage, /sdcard bind mounts
- Android env vars forwarding (ANDROID_ART_ROOT, ANDROID_DATA, etc.)
- Fake kernel release string

## Acceptance Criteria

- [ ] Script runs under busybox ash without Termux
- [ ] `proot-distro.sh list` shows available distros
- [ ] `proot-distro.sh install alpine` downloads and extracts Alpine rootfs
- [ ] `proot-distro.sh login alpine` opens a working shell
- [ ] `proot-distro.sh remove alpine` cleanly deletes rootfs
- [ ] `proot-distro.sh install debian` works with post-install setup
- [ ] `--isolated` mode works without host bind mounts
- [ ] No references to `com.termux` or `@TERMUX_@` remain in the script
