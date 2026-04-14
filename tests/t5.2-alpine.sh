#!/system/bin/sh
# Device-side: bootstrap verification only (no network)
# Runs via: adb shell "run-as id.or.oo.pr sh /data/local/tmp/t5.2-alpine.sh"

PKG="id.or.oo.pr"
DATA_DIR="/data/data/$PKG/files/usr"
HOME_DIR="/data/data/$PKG/files/home"

G="\033[0;32m"
R="\033[0;31m"
Y="\033[0;33m"
N="\033[0m"

pass=0
fail=0

ok() { pass=$((pass+1)); echo "${G}PASS${N}: $1"; }
nok() { fail=$((fail+1)); echo "${R}FAIL${N}: $1"; }
info() { echo "       $1"; }

export APP_PREFIX="$DATA_DIR"
export APP_HOME="$HOME_DIR"
export APP_PACKAGE="$PKG"
export PATH="/system/bin:/system/xbin:$DATA_DIR/bin"
export PROOT_NO_SECCOMP=1
export TERM=xterm-256color
export LANG=en_US.UTF-8
export HOME="$HOME_DIR"

# ---- Step 1: Verify bootstrap ----
echo ""
echo "=== Step 1: Verify bootstrap ==="

test -f "$DATA_DIR/bin/busybox" && ok "busybox exists" || nok "busybox missing"
test -L "$DATA_DIR/bin/bash" && ok "bash symlink exists" || nok "bash symlink missing"
test -f "$DATA_DIR/bin/bash.bin" && ok "bash.bin binary exists" || nok "bash.bin binary missing"
test -f "$DATA_DIR/bin/proot" && ok "proot exists" || nok "proot missing"
test -x "$DATA_DIR/bin/proot" && ok "proot is executable" || nok "proot not executable"
test -f "$DATA_DIR/scripts/proot-distro.sh" && ok "proot-distro.sh exists" || nok "proot-distro.sh missing"
test -L "$DATA_DIR/bin/sh" && ok "sh symlink exists" || nok "sh symlink missing"
test -L "$DATA_DIR/bin/realpath" && ok "realpath symlink exists" || nok "realpath missing"
test -L "$DATA_DIR/bin/basename" && ok "basename symlink exists" || nok "basename missing"
test -L "$DATA_DIR/bin/wget" && ok "wget symlink exists" || nok "wget missing"
test -L "$DATA_DIR/bin/tar" && ok "tar symlink exists" || nok "tar missing"
test -f "$DATA_DIR/bin/bootstrap.sh" && ok "bootstrap.sh exists" || nok "bootstrap.sh missing"
test -f "$DATA_DIR/.bootstrapped" && ok "bootstrap marker exists" || nok "bootstrap marker missing"

plugin_count=$(ls "$DATA_DIR/etc/proot-distro/"*.sh 2>/dev/null | wc -l)
if [ "$plugin_count" -ge 14 ]; then
    ok "distro plugins: $plugin_count"
else
    nok "distro plugins: $plugin_count (expected 14+)"
fi

symlink_count=$(ls -la "$DATA_DIR/bin/" 2>/dev/null | grep -c "^l" || echo "0")
if [ "$symlink_count" -gt 100 ]; then
    ok "busybox symlinks: $symlink_count"
else
    nok "busybox symlinks: $symlink_count (expected 300+)"
fi

info ""
info "proot-distro list:"
$DATA_DIR/bin/bash $DATA_DIR/scripts/proot-distro.sh list 2>&1 | sed 's/^/       /'

# ---- Summary ----
echo ""
echo "========================================="
echo "  T5.2 Bootstrap Verification Results"
echo "========================================="
echo "  ${G}PASS${N}: $pass"
echo "  ${R}FAIL${N}: $fail"
echo "========================================="

if [ "$fail" -gt 0 ]; then
    echo ""
    echo "  Check logcat: adb logcat -d -s PR:V"
fi

exit $fail
