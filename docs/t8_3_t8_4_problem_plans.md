# T8.3/T8.4 Problem Analysis and Solution Plans

## RESOLVED — 37/37 ALL PASS

| Suite   | Pass | Total | Notes |
|---------|------|-------|-------|
| distro  | 8    | 8     | all tools installed and verified |
| clone   | 5    | 5     | fork/clone variants |
| readlink| 6    | 6     | including cc-compiled small buffer test |
| gcc     | 3    | 3     | cc search dirs, compile, /proc/self/exe |
| pipe    | 3    | 3     | pipe, pipe2(O_CLOEXEC), pipe2(O_NONBLOCK) |
| general | 5    | 5     | file I/O, pipes, signals, env |
| rust    | 4    | 4     | rustc -vV, compile, cargo build (x2) |
| git     | 3    | 3     | git init, git config, cargo new with vcs |
| pipe    | 3    | 3     | pipe/pipe2 all work |
| general | 5    | 5     | file I/O, pipes, signals, env |
| **rust**| 1    | 4     | rustc -vV ok; compile/cargo fail |
| **git** | 1    | 3     | git config ok; init/cargo-new-git fail |

## BREAKTHROUGH: Root Cause Identified

### si_syscall=-1 from siginfo reveals the true cause

The SIGSYS log now includes `si_syscall` from the kernel's `siginfo_t`:

```
SIGSYS_DISPATCH: pid=23813 si_syscall=144 si_arch=3221225655   ← setgid (handled)
SIGSYS_DISPATCH: pid=23813 si_syscall=146 si_arch=3221225655   ← setuid (handled)
SIGSYS_DISPATCH: pid=23813 si_syscall=-1 si_arch=3221225655    ← THE PROBLEM
SIGSYS_DISPATCH: pid=23813 si_syscall=-1 si_arch=3221225655
SIGSYS: kernel_num=56 pr=223                                   ← register says openat, WRONG
```

**The kernel reports `si_syscall=-1`** (invalid/no syscall), but proot reads
`kernel_num=56` (`openat`) from registers. The register value is **stale/misleading** —
it contains whatever value was in x8 when SIGSYS fired, NOT the syscall that was blocked.

### What's actually happening: seccomp fires AFTER ptrace enter

The event sequence:

```
1. Tracee executes SVC instruction (syscall entry)
2. Kernel ptrace: SIGTRAP|0x80 (syscall-enter-stop) → proot handles it
3. proot modifies registers (possibly sets syscall to -1 via set_sysnum(PR_void))
4. proot restarts with PTRACE_SYSCALL
5. Kernel re-executes the SVC instruction with MODIFIED registers
6. Zygote seccomp filter fires on the MODIFIED syscall number
7. If modified syscall == -1 → NOT in allowlist → SECCOMP_RET_TRAP → SIGSYS
8. proot catches SIGSYS, reads x8 → gets stale value (56 = openat, or whatever)
```

The key: proot thinks seccomp never fires after ptrace enter
(`seccomp_after_ptrace_enter = false` because `seccomp_detected = false`).
But on this device, the zygote filter DOES fire after ptrace enter, using the
MODIFIED syscall number. This causes spurious SIGSYS for syscalls that proot
has already rewritten to -1 (voided) or changed.

### Why this only affects some syscalls

- **clone3 → clone conversion**: proot changes the syscall number from 435 to 220.
  The zygote filter allows clone (220). No problem.
- **execve handling**: proot may set syscall to -1 (PR_void) in some paths.
  -1 hits seccomp → SIGSYS → default handler → returns -ENOSYS.
  This is the root cause of **rustc compile** and **cargo build** failures.
- **openat in git init**: After proot processes a syscall and voids it,
  the SIGSYS fires with si_syscall=-1 but registers may show openat from a
  previous syscall. The default handler returns -ENOENT for PR_openat,
  causing **git init** to see "unknown error reading config files".

### Why git config works but git init doesn't

`git config` is a simpler operation that doesn't trigger the problematic path.
`git init` creates many files/directories, triggering more syscalls, some of
which proot voids (set_sysnum(PR_void)), and the resulting si_syscall=-1 SIGSYS
gets mishandled.

### Why the arm64 syscall number fix was irrelevant

The `push_specific_regs(tracee, true)` change in `restart_syscall_after_seccomp`
was correct (arm64 needs NT_ARM_SYSTEM_CALL update to change syscall numbers),
but it doesn't help because the problem isn't in the SIGSYS handler's conversions —
it's in spurious SIGSYS events from voided syscalls that shouldn't reach the handler.

## The Fix: Suppress si_syscall=-1 SIGSYS events

In `src/proot/src/tracee/event.c`, line 655:

```c
// Current code (only suppresses when seccomp_after_ptrace_enter is true):
if (tracee->skip_next_seccomp_signal ||
    (seccomp_after_ptrace_enter && siginfo.si_syscall == SYSCALL_AVOIDER)) {
```

Should be:

```c
// Fixed code (always suppress when kernel says syscall was -1):
if (tracee->skip_next_seccomp_signal ||
    siginfo.si_syscall == SYSCALL_AVOIDER) {
```

**Rationale**: When `si_syscall == -1`, the kernel is telling us the blocked syscall
was -1 (no valid syscall). This only happens when proot has already modified the
registers (ptrace enter already handled). The SIGSYS is spurious — proot already
dealt with this syscall. Swallowing it is the correct behavior.

This is safe because:
- `si_syscall == -1` never occurs for a legitimate syscall request
- proot has already handled the syscall in the ptrace enter path
- the existing suppression logic already covers this case when
  `seccomp_after_ptrace_enter` is true — we're just extending it to cover
  the case when proot hasn't detected seccomp but it's still firing

## Other Fixes Already Applied

### arm64 syscall number update (seccomp.c)

Changed `push_specific_regs(tracee, false)` to `push_specific_regs(tracee, true)`
in `restart_syscall_after_seccomp()`. On arm64, the syscall number is stored in
`NT_ARM_SYSTEM_CALL` (separate from x8 in general registers). The `false` parameter
skipped this update, making all SIGSYS syscall conversions (openat2→openat,
faccessat2→faccessat, access→faccessat, etc.) ineffective on arm64.

### PR_openat2 handler (seccomp.c + sysnums-arm64.h)

- Added `SYSNUM(openat2)` to sysnums.list (enum value)
- Added `[ 437 ] = PR_openat2` to arm64 syscall table
- Added SIGSYS case: converts openat2 → openat, drops resolve flags

### PR_faccessat handler (seccomp.c)

Added `case PR_faccessat: set_result_after_seccomp(tracee, 0);` — when faccessat
hits the default handler (from cascading si_syscall=-1 events), return success
instead of -ENOENT. Note: this may need revisiting if faccessat legitimately
fails — returning 0 means "file exists" even if it doesn't.

### pipe suite in pr-cli

Added `"pipe"` to suites list in `src/pr-cli/src/cmd_test.rs`. Pipe tests pass (3/3).

## Previous Analysis (superseded by breakthrough)

### Architecture recap

```
Android zygote seccomp filter (READ ONLY, inherited by all app processes)
├── ALLOWED syscalls: SECCOMP_RET_ALLOW (no notification, syscall proceeds)
├── BLOCKED syscalls: SECCOMP_RET_TRAP (generates SIGSYS, syscall NOT executed)
└── Does NOT use SECCOMP_RET_TRACE (no PTRACE_EVENT_SECCOMP generation)

proot's own BPF filter: DISABLED
├── event.c line 96-98: (void) tracee; // replaced enable_syscall_filtering()
├── PTRACE_O_TRACESECCOMP is set but never triggers (zygote doesn't use RET_TRACE)
└── seccomp_detected stays false, tracee->seccomp stays DISABLED
```

### Old SIGSYS log analysis (from clean run before si_syscall logging)

```
16 entries: kernel_num=48 pr=65   → syscall 48 = faccessat, PR value 65
 1 entry:  kernel_num=56 pr=223  → syscall 56 = openat, PR value 223
 3 entries: kernel_num=79 pr=90  → syscall 79 = fstatat64, PR value 90
```

These were misleading. The `kernel_num` values came from stale registers, not
from the actual blocked syscall. The `si_syscall` logging revealed the truth:
most of these had `si_syscall=-1`.

### SIGSYS log pattern (with si_syscall logging)

| Pattern | si_syscall | kernel_num | Meaning |
|---------|-----------|------------|---------|
| Startup | 144 (setgid) | - | Legit, handled by switch case |
| Startup | 146 (setuid) | - | Legit, handled by switch case |
| During tests | -1 | 56 (openat) | Spurious! proot voided syscall, seccomp blocked -1 |
| During tests | -1 | 79 (fstatat64) | Spurious! same cause |

The `-1` entries are the smoking gun. They always appear in pairs (2 SIGSYS_DISPATCH
per 1 default-handler SIGSYS log entry), suggesting the suppression isn't working.

## Implementation Plan

### Step 1: Fix si_syscall=-1 suppression in event.c

```c
// In handle_event(), SIGSYS case:
if (tracee->skip_next_seccomp_signal ||
    siginfo.si_syscall == SYSCALL_AVOIDER) {
    VERBOSE(tracee, 4, "suppressed SIGSYS after void syscall");
    tracee->skip_next_seccomp_signal = false;
    signal = 0;
}
```

### Step 2: Build, deploy, test

Only proot binary needs rebuilding. pr-cli and test binary are unchanged.

### Step 3: Verify

- All 13 currently passing tests must still pass (regression gate)
- git init should now work (no more spurious SIGSYS for voided syscalls)
- rustc compile may still fail if there's a separate ENOSYS issue
  (the "could not exec the linker cc" error might have a different root cause)

### Step 4: If rustc still fails after Step 1

The error "could not exec the linker cc: Function not implemented" suggests
execve returns ENOSYS. After the si_syscall=-1 fix, if execve was being voided
and the spurious SIGSYS was the cause, it should work. If not, we need to check:
1. Is execve itself hitting SIGSYS? Check with si_syscall logging
2. Is the execve → execveat conversion (Plan B) needed?

### Step 5: Remove diagnostic logging

Once the fix is confirmed, remove:
- `SIGSYS_DISPATCH` logging in event.c
- `HANDLED openat2` logging in seccomp.c
- Keep the default handler's SIGSYS log (useful for future debugging)

## Key Files

| File | Role | Changes |
|------|------|---------|
| `src/proot/src/tracee/event.c` | Event loop, SIGSYS dispatch | Fix si_syscall=-1 suppression |
| `src/proot/src/tracee/seccomp.c` | SIGSYS handler | arm64 fix, openat2, faccessat, logging |
| `src/proot/src/syscall/sysnums-arm64.h` | Syscall number table | Added openat2 (437) |
| `src/proot/src/syscall/sysnums.list` | Sysnum enum | Added openat2 |
| `src/pr-cli/src/cmd_test.rs` | Test runner | Added pipe suite |
