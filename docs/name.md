# Name: pr

**pr** stands for **PRoot** — which is short for **ptrace-based root**.

PRoot is a user-space implementation of `chroot`, `mount --bind`, and `binfmt_misc` that uses Linux's `ptrace()` system call to intercept and translate filesystem paths. This allows running Linux distributions inside a directory without actual root privileges — exactly what this app does on Android.

The app package `id.or.oo.pr` inherits the name: a standalone Android APK that runs Linux distros via proot, with no Termux dependency.
