# Proposal: Initial Implementation of Standalone proot-distro for Android

## Problem

Running Linux distributions on Android typically requires Termux as a runtime
environment. Users who want a standalone solution — or who want to embed
Linux-in-Android capabilities into their own app — have no option that doesn't
pull in the entire Termux stack (its package manager, prefix hierarchy,
LD_PRELOAD linker hooks, and app-specific paths).

## Solution

Port proot-distro into a fully standalone Android package (`id.or.oo.pr`)
that bundles:

1. **A patched proot binary** — vanilla proot with cherry-picked Android
   patches from termux-proot (SIGSYS/seccomp handler, ARM64 support,
   link2symlink, ashmem_memfd, statx, kill-on-exit, sysvipc, f2fs workaround,
   fix_symlink_size, hidden_files, port_switch).
2. **A ported proot-distro.sh** — forked from termux-proot-distro v4.38.0 with
   all Termux dependencies removed, using app-private paths and a bundled
   static busybox for runtime utilities.
3. **An Android APK** — Java/Kotlin app shell with embedded terminal emulator,
   JNI launcher for proot, and self-initialization on first run.
4. **Distro plugins** — reused from termux-proot-distro (tarballs are
   distribution-agnostic).

## Domain & Repository

- Domain: `pr.oo.or.id`
- Repository: `github.com/oonid/pr`
- Android package ID: `id.or.oo.pr`

## Scope

### In Scope

- Cherry-pick P0–P3 patches from termux-proot into vendor/proot
- NDK build pipeline producing static proot for aarch64 (and arm)
- Ported proot-distro.sh (all Termux references removed)
- Static busybox integration (bundled, with applet symlinks)
- Android APK with terminal emulator, distro manager UI, and JNI proot launcher
- Distro plugins for Alpine, Debian, Ubuntu, Arch Linux, Fedora
- Install, login, remove, list, backup, restore, rename, reset, copy commands
- Non-isolated and isolated execution modes
- Fake /proc and /sys data generation
- Android UID/GID registration in guest

### Out of Scope (v1)

- QEMU user-mode binary bundling (foreign architecture emulation)
- Blink emulator support
- `DISTRO_TYPE="termux"` — Termux bootstrap is not relevant
- FUSE mounting support
- Appimage / Flatpak / Snap support
- Google Play Store distribution (sideload only initially)

## Key Decisions

| Decision | Choice | Rationale |
|---|---|---|
| proot source base | Vanilla + cherry-picks | Clean upstream tracking; minimal patch surface |
| Runtime utilities | Static busybox | Single binary, small footprint, all needed applets |
| Deployment target | Android APK | Self-contained, no root required |
| Termux dependency | None | All paths, packages, assumptions removed |
| Data location | App-private `/data/data/id.or.oo.pr/files/` | Standard Android app data, no special permissions |
| Min Android version | API 28 (Android 9) | Modern security model, broad device coverage |
| Rootfs hosting | Existing easycli.sh tarballs + self-hosted mirror at pr.oo.or.id | Reuse existing rootfs images; add fallback mirror |

## References

- Exploration report: `docs/porting-exploration.md`
- Upstream proot: `vendor/proot/`
- Termux proot (reference): `vendor/termux-proot/`
- Termux proot-distro (reference): `vendor/termux-proot-distro/`
