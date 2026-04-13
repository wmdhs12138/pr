# Spec: Busybox Integration

## Capability

Bundle a statically-linked busybox binary that provides all runtime utilities
needed by proot-distro.sh and the overall system, replacing the need for
individually-installed GNU coreutils, curl, tar, etc.

## Requirements

### REQ-BB-001: Binary Source

Obtain a static busybox binary for aarch64. Options:

1. **Pre-built**: Download from busybox.net official builds or trusted Android
   tool projects (e.g., `topjohnwu/magisk` busybox)
2. **Built from source**: Clone busybox git, configure with `make defconfig`,
   enable static linking, cross-compile with Android NDK

### REQ-BB-002: Required Applets

Busybox must be compiled with (or verified to include) these applets:

**Core (used by proot-distro.sh):**
`awk`, `basename`, `bzip2`, `cat`, `chmod`, `cp`, `cut`, `du`, `find`, `grep`,
`gzip`, `head`, `id`, `mkdir`, `rm`, `sed`, `tar`, `xargs`, `xz`, `sha256sum`,
`file`, `wget`, `mknod`, `chown`, `env`, `false`, `ln`, `ls`, `mv`, `printf`,
`pwd`, `readlink`, `realpath`, `stat`, `touch`, `tr`, `true`, `uname`, `wc`,
`which`

**Additional (useful in guest interaction):**
`dd`, `expr`, `test`, `[`, `[[`, `ash`, `sh`, `sleep`, `kill`, `ps`, `date`,
`hostname`, `mktemp`, `nohup`, `tee`, `uniq`, `sort`, `diff`, `patch`,
`md5sum`, `sha1sum`, `od`, `hexdump`, `strings`, `df`, `free`, `top`

### REQ-BB-003: Applet Symlinks

On initialization, create symlinks in `${APP_PREFIX}/bin/` for each applet:

```
${APP_PREFIX}/bin/sh -> busybox
${APP_PREFIX}/bin/bash -> busybox
${APP_PREFIX}/bin/tar -> busybox
${APP_PREFIX}/bin/grep -> busybox
${APP_PREFIX}/bin/sed -> busybox
${APP_PREFIX}/bin/awk -> busybox
...
```

This is done via: `busybox --list | while read applet; do ln -sf busybox ${APP_PREFIX}/bin/$applet; done`

### REQ-BB-004: Download Support

`busybox wget` replaces `curl` for downloading rootfs tarballs. The script must
be adapted to use `wget -O <output> <url>` instead of `curl -Lo <output> <url>`.

Limitations of busybox wget to handle:
- No automatic HTTPS certificate pinning (acceptable for rootfs downloads)
- No retry logic (wrap in a retry loop in the script)
- Progress output format differs from curl

### REQ-BB-005: tar Compatibility

Verify that busybox `tar` can extract all distro rootfs formats:
- `.tar.xz` (Alpine, Debian, Ubuntu, Arch, Fedora)
- `.tar.gz`
- `.tar.bz2`

Known busybox tar limitations:
- May not handle certain xattr/acl entries
- May not handle some GNU tar extensions

**Fallback:** If busybox tar proves insufficient for specific distros, bundle a
static GNU tar binary as `${APP_PREFIX}/bin/gnutar` and use it for extraction.

### REQ-BB-006: file Command Compatibility

Busybox `file` provides limited ELF detection compared to GNU file. The
`detect_cpu_arch()` function must work with busybox's output format:

Busybox output: `ELF 64-bit LSB executable, ARM aarch64, ...`
GNU file output: `ELF 64-bit LSB pie executable, ARM aarch64, version 1 (SYSV), ...`

The regex in `detect_cpu_arch` must handle both formats. Test with:
```
busybox file /bin/ls
busybox file /path/to/rootfs/usr/bin/sh
```

## Acceptance Criteria

- [ ] Static busybox binary is < 3MB
- [ ] All required applets are available (`busybox --list` contains all)
- [ ] `busybox wget` can download a file from easycli.sh
- [ ] `busybox tar xf alpine-*.tar.xz` extracts correctly
- [ ] `busybox file` identifies aarch64 ELF correctly
- [ ] All applet symlinks are created during initialization
- [ ] proot-distro.sh works using only busybox-provided utilities
