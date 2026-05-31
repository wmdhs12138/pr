# Spec: proot Binary Build

## Capability

Build a statically-linked proot binary for Android aarch64 (and optionally arm)
from vanilla proot source with cherry-picked Android patches from termux-proot.

## Requirements

### REQ-PROOT-001: Cherry-pick Android Patches

Apply patches from `vendor/termux-proot/` into `src/proot/` (working copy of `vendor/proot/`) in priority order:

**P0 (blocking):**
- SIGSYS/seccomp handler: `src/tracee/seccomp.c`, `src/tracee/seccomp.h`
- ARM64 arch support + POKEDATA workaround: `src/arch.h`, `src/tracee/mem.c`, `src/loader/assembly-arm64.h`
- link2symlink extension: `src/extension/link2symlink/`

**P1 (strongly recommended):**
- ashmem_memfd extension: `src/extension/ashmem_memfd/`
- statx() support: `src/tracee/statx.c`, `src/tracee/statx.h`
- --kill-on-exit option: modifications to `src/cli/proot.c`, `src/tracee/tracee.h`, `src/tracee/event.c`

**P2 (quality of life):**
- sysvipc extension: `src/extension/sysvipc/`
- f2fs bug workaround: `src/path/f2fs-bug.c`, `src/path/f2fs-bug.h`
- fix_symlink_size extension: `src/extension/fix_symlink_size/`
- hidden_files extension: `src/extension/hidden_files/`
- port_switch extension: `src/extension/port_switch/`
- mountinfo extension: `src/extension/mountinfo/`

**P3 (compatibility):**
- is_aarch32 support in tracee.h
- Enhanced syscall chaining fields in tracee.h
- Seccomp filter additions in src/syscall/seccomp.c

### REQ-PROOT-002: GNUmakefile Updates

Update `src/proot/src/GNUmakefile` to:
- Add all new object files from cherry-picked patches
- Use `?=` (conditional assignment) for CC, STRIP, OBJCOPY, OBJDUMP
- Support cross-compilation via `CROSS_COMPILE` or direct CC override
- Link `libtalloc` directly via `-ltalloc` (not pkg-config)
- Include ARM64-specific loader build rules

### REQ-PROOT-003: NDK Build Pipeline

Create a build script (`build.sh`) that:
1. Sets up Android NDK standalone toolchain for aarch64 (API 28)
2. Cross-compiles libtalloc as a static library
3. Builds proot with NDK toolchain
4. Strips the binary
5. Verifies the binary is a static ELF for aarch64
6. Optionally repeats for arm (armeabi-v7a)

### REQ-PROOT-004: Supported CLI Options

The built proot binary must support these options (beyond upstream):
- `--link2symlink` / `-l`
- `--ashmem-memfd`
- `--sysvipc`
- `--kill-on-exit`
- `-L` (fix symlink size)
- `-H` (hide .proot.* files)
- `-p` (port switching)
- `--kernel-release=`
- `--change-id=`
- `--root-id`

### REQ-PROOT-005: Build Output

- Binary name: `proot`
- Architecture: `aarch64` (primary), `arm` (secondary)
- Linking: static (no dynamic dependencies at runtime)
- Size target: < 3MB per binary

## Acceptance Criteria

- [ ] proot binary runs on Android aarch64 device without crashes
- [ ] `proot --link2symlink --root-id -r /some/rootfs /bin/sh` succeeds
- [ ] SIGSYS from blocked syscalls is handled correctly (no crash on open, stat, etc.)
- [ ] Hard link emulation works (link2symlink)
- [ ] memfd_create falls back to ashmem on older kernels
- [ ] All tracees are killed on proot exit (--kill-on-exit)
