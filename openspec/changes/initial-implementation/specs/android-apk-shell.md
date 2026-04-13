# Spec: Android APK Shell

## Capability

An Android application that bundles the proot binary, busybox, proot-distro
script, and distro plugins; provides a UI for managing distributions and an
embedded terminal for interacting with guest Linux sessions.

## Requirements

### REQ-APP-001: Application Identity

- **Package ID**: `id.or.oo.pr`
- **App name**: "PR" (or user-configurable)
- **Min SDK**: 28 (Android 9 Pie)
- **Target SDK**: 35

### REQ-APP-002: Bundled Assets

The APK must contain these assets, extracted to app-private storage on first
launch:

| Asset | Target Location |
|---|---|
| `lib/arm64-v8a/proot.so` | `${files}/usr/bin/proot` |
| `assets/bin/busybox-arm64` | `${files}/usr/bin/busybox` |
| `assets/scripts/proot-distro.sh` | `${files}/usr/scripts/proot-distro.sh` |
| `assets/plugins/*.sh` | `${files}/usr/etc/proot-distro/*.sh` |

### REQ-APP-003: First-Run Initialization

On first launch (`BootstrapService`):

1. Create directory structure under `/data/data/id.or.oo.pr/files/`:
   ```
   usr/bin/
   usr/etc/proot-distro/
   usr/var/lib/proot-distro/dlcache/
   usr/var/lib/proot-distro/installed-rootfs/
   usr/scripts/
   home/
   tmp/
   ```
2. Copy proot from native library directory to `usr/bin/proot`; `chmod 755`
3. Copy busybox from assets to `usr/bin/busybox`; `chmod 755`
4. Create busybox applet symlinks in `usr/bin/` for all required utilities
5. Copy proot-distro.sh from assets to `usr/scripts/`; `chmod 755`
6. Copy distro plugins from assets to `usr/etc/proot-distro/`; `chmod 600`
7. Set `initialized = true` in SharedPreferences
8. Skip on subsequent launches

### REQ-APP-004: Main Activity UI

The main screen shows:

1. List of available distributions (from plugins)
2. Install status for each (installed / not installed / installing)
3. "Install" button for uninstalled distros
4. "Login" button for installed distros
5. "Remove" button for installed distros (with confirmation dialog)
6. "Settings" gear icon

### REQ-APP-005: Terminal Activity

A full-screen terminal emulator that:

1. Executes `proot-distro.sh login <distro>` via `Runtime.exec()` or `ProcessBuilder`
2. Wires process stdin/stdout/stderr to the terminal view
3. Handles terminal resize (SIGWINCH)
4. Shows session name / distro name in action bar
5. Handles process exit gracefully (return to main screen)

The terminal view can use:
- Termux's `TerminalView` (Apache 2.0, from `jackpal.androidterm` or `termux/termux-app`)
- Or any compatible terminal emulator View

### REQ-APP-006: ProotLauncher

Native bridge component that:

1. Constructs the execution environment:
   ```
   APP_PREFIX=/data/data/id.or.oo.pr/files/usr
   APP_HOME=/data/data/id.or.oo.pr/files/home
   APP_PACKAGE=id.or.oo.pr
   PATH=/data/data/id.or.oo.pr/files/usr/bin
   HOME=/data/data/id.or.oo.pr/files/home
   ```
2. Executes: `/data/data/id.or.oo.pr/files/usr/scripts/proot-distro.sh login <distro> [options]`
3. Returns the `Process` object for I/O wiring

### REQ-APP-007: Permissions

| Permission | Required | Purpose |
|---|---|---|
| `INTERNET` | Yes | Download rootfs tarballs |
| `MANAGE_EXTERNAL_STORAGE` | Optional | Access /sdcard in non-isolated mode |
| `WRITE_EXTERNAL_STORAGE` | Optional | Legacy storage access (pre-Android 11) |
| `FOREGROUND_SERVICE` | Yes | Keep terminal session alive |

### REQ-APP-008: Distro Plugins (Bundled)

Initial set of bundled plugins:

| Plugin | Status |
|---|---|
| `alpine.sh` | Included |
| `debian.sh` | Included |
| `ubuntu.sh` | Included |
| `archlinux.sh` | Included |
| `fedora.sh` | Included |

Additional plugins can be added by placing `.sh` files in
`${files}/usr/etc/proot-distro/`.

## Acceptance Criteria

- [ ] App installs and launches on Android 9+ device (aarch64)
- [ ] First-run initialization completes without errors
- [ ] Main screen shows 5 available distributions
- [ ] "Install Alpine" downloads and installs successfully
- [ ] "Login Alpine" opens terminal with working shell
- [ ] `uname -a` in guest shows fake kernel version `6.17.0-pr`
- [ ] `apt update` works in Debian/Ubuntu
- [ ] "Remove Alpine" deletes rootfs cleanly
- [ ] App survives device rotation and background/foreground cycle
