# Important Notes

See also:
- `docs/phase6.md` — Phase 6: Replace proot-distro.sh with Rust binary (exploration, viability tests, recommendation)
- `docs/phase5-alpine.md` — Phase 5: Alpine Linux investigation and fixes

---

## Android W^X / SELinux / targetSdk Restrictions

Date: 2026-04-15
Device: Samsung, Android 16 (SDK 36), aarch64 ()
Context: Discovered during T5.2/T6.9 integration testing

### What is W^X

Android enforces Write-XOR-Execute (W^X) policy on app processes. The kernel blocks `execve()` on files located in app-writable directories (labeled `app_data_file` by SELinux). This prevents apps from downloading and running arbitrary code.

### The Three Blockers (targetSdk 29+)

When proot runs from the app process (`u:r:untrusted_app:s0`) with targetSdk >= 29:

```
proot error: execve("/bin/sh"): Permission denied            ← W^X blocks execve on app_data_file
proot error: can't chmod '/data/.../proot-XXXX': Function not implemented  ← seccomp ENOSYS
proot error: can't chdir to '/': Function not implemented     ← seccomp ENOSYS
```

1. **execve Permission denied**: Proot's child tries to exec `/bin/sh` which resolves to the rootfs in `/data/data/...`. SELinux blocks it because the file has the `app_data_file` label.

2. **chmod ENOSYS**: Proot's tracer code needs to `chmod()` a temp file. The Android zygote's seccomp filter returns ENOSYS (Function not implemented).

3. **chdir ENOSYS**: Proot's tracer code needs to `chdir()` into the rootfs. Same seccomp filter blocks it.

### Why run-as Works But App Doesn't

```
run-as id.or.oo.pr   → SELinux context: u:r:runas_app:s0:c3,c258,c512,c768  → proot works
app process          → SELinux context: u:r:untrusted_app:s0:c3,c258,c512,c768  → proot blocked
```

`runas_app` has different SELinux policies than `untrusted_app`. The W^X and seccomp restrictions only apply to `untrusted_app`.

### targetSdk Threshold

Tested on device:

| targetSdk | proot login | Notes |
|-----------|-------------|-------|
| 28 | **WORKS** | No W^X enforcement. Proot can execve, chmod, chdir freely. |
| 29 | **FAILS** | W^X + seccomp enforced. All three blockers appear. |
| 35 | **FAILS** | Same as 29+. |

**Conclusion**: targetSdk 28 is the maximum for proot to function. This matches Termux's approach (they also use targetSdk 28).

### nativeLibraryDir is the Exception

Files in the APK's native library directory can be executed even from `untrusted_app`:

```
/data/app/~~<random>/<package>-<random>/lib/arm64/  → SELinux allows execve()
/data/data/<package>/files/                          → SELinux denies execve()
```

This is why `libproot.so`, `libbusybox.so`, `libpr-cli.so` can all be exec'd — they're in nativeLibraryDir. But the rootfs binaries in `/data/data/.../installed-rootfs/` cannot.

### Why Proot Bind Mounts Don't Help

Proot's `--bind` is virtual — it only translates paths in ptrace syscall interception. When the kernel actually performs the `execve()`, it sees the real filesystem path (in `/data/data/...`), not the virtual guest path. So binding `libbusybox.so:/bin/sh` doesn't help — the kernel still tries to exec the real rootfs file.

### Why the Fork Matters

The forkPty() JNI function forks the app process. The child inherits `untrusted_app` SELinux context AND the zygote's seccomp filter. This is different from `run-as` which creates a new process in `runas_app` context.

### Implications for Future Android Versions

- **Android 15+**: Google may further restrict ptrace or native library execution
- **Samsung Knox**: Some firmware mounts `/data/data/<pkg>/files/` with `noexec` flag
- **Yama ptrace_scope**: Must be 0 or 1 for proot to work (Samsung defaults to 1, which allows parent→child ptrace)

### Potential Workarounds (Future Research)

1. **Static ELF Loader** (`libexecloader.so`): A custom no_std Rust binary in nativeLibraryDir that maps target ELF into memory via mmap (not execve). Proot rewrites `execve("/bin/sh")` to `execve("libexecloader.so", ["/bin/sh"])`. Solves the execve blocker but NOT the chmod/chdir seccomp blockers.

2. **Guest dynamic linker as proxy**: Copy `ld-musl-aarch64.so.1` into nativeLibraryDir. Proot rewrites execve to go through the linker. Unlikely to work because the linker needs a full Linux ABI.

3. **Lower targetSdk to 28**: Current working solution. Same as Termux. May limit Play Store distribution.

### Zygote Seccomp Filter (targetSdk 28, aarch64)

Even at targetSdk=28, the Android zygote installs a BPF seccomp filter on the app process
(`Seccomp: 2, Seccomp_filters: 1`). This filter persists even with `PROOT_NO_SECCOMP=1`
(which only disables proot's own seccomp filter, not the zygote's).

**Blocked syscalls** (trigger SIGSYS from zygote filter):

| Syscall | Number | Handler | Strategy |
|---------|--------|---------|----------|
| `getcwd` | 17 | `PR_getcwd` | Read `tracee->fs->cwd`, write to tracee buffer, return length |
| `chdir` | 49 | `PR_chdir` | Translate path, update `tracee->fs->cwd`, return 0 |
| `fchdir` | 50 | `PR_fchdir` | Resolve dirfd to path, update cwd, return 0 |
| `linkat` | 37 | `PR_linkat` | Try `renameat()`, fallback to copy+delete on EXDEV/EACCES |
| `renameat2` | 276 | `PR_renameat2` | Downgrade to `renameat` (drop flags) and restart |
| `faccessat2` | 439 | `PR_faccessat2` | Downgrade to `faccessat` (drop flags) and restart |
| `process_madvise` | 440 | `PR_process_madvise` | Return 0 (noop — advisory) |
| `setgid` | 144 | default | Return -ENOSYS (harmless) |
| `setuid` | 146 | default | Return -ENOSYS (harmless) |
| `memfd_create` | 279 | default | Return -ENOSYS |
| `openat` | 56 | default | Return **-ENOENT** (not -ENOSYS) |
| `fstatat64` | 79 | default | Return **-ENOENT** (not -ENOSYS) |

Note: original enosys_test incorrectly used 281 for process_madvise; 281 is execveat on arm64.

**aarch64 x0 clobber bug**: On aarch64, `SYSARG_1` and `SYSARG_RESULT` both map to register
x0. At SIGSYS time the kernel may clobber x0 before proot reads it. All handlers that read
`SYSARG_1` use `ORIGINAL` register version (saved right after `fetch_regs()`) instead of
`CURRENT`. `SYSARG_2`-`SYSARG_6` (x1-x5) are unaffected. Without this fix, `fchdir(fd)`
appeared as `fchdir(0)` because x0 was clobbered to 0.

**openat/fstatat64 ENOENT fix**: The SIGSYS `default:` handler returns `-ENOENT` instead of
`-ENOSYS` for these two syscalls. This is critical because musl's ldso `path_open()` treats
ENOENT as "continue searching" but ENOSYS as "abort all search". Without this fix, a single
failed openat on a non-existent path would prevent the dynamic linker from finding the real
library.

**Verified working** (on fresh Alpine install, app process with seccomp: 2):
- `apk update`, `apk add vim`, `apk add curl`, `apk add openssh` — all 0 errors
- `vim --version`, `curl --version`, `ssh -V` — all pass

### Key Files

- `android/app/build.gradle.kts` — `targetSdk = 28` (MUST NOT exceed 28 for proot to work)
- `src/proot/src/tracee/event.c` — seccomp patch (line 95: skip proot's own seccomp filter)
- `src/proot/src/tracee/seccomp.c` — proot's seccomp handler (12 SIGSYS handlers, x0 clobber fix, ENOENT for openat/fstatat64)
- `src/pr-cli/src/login.rs` — proot invocation with PROOT_TMP_DIR, arg0("proot"), nativeLibraryDir paths
- `docs/phase5-alpine.md` — Alpine-specific investigation and seccomp findings

---

## Project Name: pr

**pr** stands for **PRoot** — which is short for **ptrace-based root**.

PRoot is a user-space implementation of `chroot`, `mount --bind`, and `binfmt_misc` that uses Linux's `ptrace()` system call to intercept and translate filesystem paths. This allows running Linux distributions inside a directory without actual root privileges — exactly what this app does on Android.

The app package `id.or.oo.pr` inherits the name: a standalone Android APK that runs Linux distros via proot, with no Termux dependency.
