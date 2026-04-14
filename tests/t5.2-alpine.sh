#!/bin/sh
set -e

PKG="id.or.oo.pr"
DATA_DIR="/data/data/$PKG/files/usr"
HOME_DIR="/data/data/$PKG/files/home"
APK="android/app/build/outputs/apk/debug/app-debug.apk"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

pass=0
fail=0
skip=0

log_pass() { pass=$((pass + 1)); echo "${GREEN}PASS${NC}: $1"; }
log_fail() { fail=$((fail + 1)); echo "${RED}FAIL${NC}: $1"; }
log_skip() { skip=$((skip + 1)); echo "${YELLOW}SKIP${NC}: $1"; }
log_info() { echo "       $1"; }

# ---- Step 0: Install APK ----
echo ""
echo "=== Step 0: Install APK ==="
if [ ! -f "$APK" ]; then
    log_fail "APK not found at $APK"
    echo "Run: cd android && ANDROID_HOME=/home/o/Android/Sdk ./gradlew assembleDebug"
    exit 1
fi

adb install -r "$APK" 2>&1 | tail -1
log_pass "APK installed"

# ---- Step 1: Launch app (triggers bootstrap) ----
echo ""
echo "=== Step 1: Bootstrap ==="
adb shell am start -n "$PKG/.MainActivity" > /dev/null 2>&1
log_info "Waiting for bootstrap to complete..."
sleep 5

# Check bootstrap via logcat
adb logcat -d -s "PR:V" "BootstrapService:V" | tail -20

# Check critical files exist via run-as
check_file() {
    adb shell "run-as $PKG ls -la $DATA_DIR/$1 2>/dev/null" | grep -q "$(basename $1)" 2>/dev/null
}

echo ""
echo "--- Checking bootstrap files ---"

if check_file "bin/busybox"; then
    log_pass "busybox exists"
else
    log_fail "busybox missing"
fi

if check_file "bin/bash"; then
    log_pass "bash exists"
else
    log_fail "bash missing"
fi

if check_file "bin/proot"; then
    log_pass "proot exists"
else
    log_fail "proot missing"
fi

if check_file "scripts/proot-distro.sh"; then
    log_pass "proot-distro.sh exists"
else
    log_fail "proot-distro.sh missing"
fi

plugin_count=$(adb shell "run-as $PKG ls $DATA_DIR/etc/proot-distro/*.sh 2>/dev/null" | grep -c "\.sh" 2>/dev/null || echo "0")
if [ "$plugin_count" -ge 14 ]; then
    log_pass "distro plugins present ($plugin_count)"
else
    log_fail "distro plugins count: $plugin_count (expected 14+)"
fi

symlink_count=$(adb shell "run-as $PKG ls $DATA_DIR/bin/sh 2>/dev/null" | grep -c "sh" 2>/dev/null || echo "0")
if [ "$symlink_count" -ge 1 ]; then
    log_pass "busybox symlinks created (sh exists)"
else
    log_fail "busybox symlinks missing (sh not found)"
fi

# ---- Step 2: Install Alpine ----
echo ""
echo "=== Step 2: Install Alpine ==="

adb shell "run-as $PKG \
    env -i \
    APP_PREFIX=$DATA_DIR \
    APP_HOME=$HOME_DIR \
    APP_PACKAGE=$PKG \
    PATH=$DATA_DIR/bin:/system/bin:/system/xbin \
    HOME=$HOME_DIR \
    PROOT_NO_SECCOMP=1 \
    TERM=xterm-256color \
    LANG=en_US.UTF-8 \
    $DATA_DIR/bin/bash $DATA_DIR/scripts/proot-distro.sh install alpine" 2>&1

echo ""

if adb shell "run-as $PKG test -d $DATA_DIR/var/lib/proot-distro/installed-rootfs/alpine && echo EXISTS" 2>/dev/null | grep -q "EXISTS"; then
    log_pass "Alpine rootfs installed"
else
    log_fail "Alpine rootfs not found"
fi

# ---- Step 3: Login and verify shell ----
echo ""
echo "=== Step 3: Login to Alpine ==="

login_output=$(adb shell "run-as $PKG \
    env -i \
    APP_PREFIX=$DATA_DIR \
    APP_HOME=$HOME_DIR \
    APP_PACKAGE=$PKG \
    PATH=$DATA_DIR/bin:/system/bin:/system/xbin \
    HOME=$HOME_DIR \
    PROOT_NO_SECCOMP=1 \
    TERM=xterm-256color \
    LANG=en_US.UTF-8 \
    $DATA_DIR/bin/bash -c 'echo LOGIN_TEST_START; \
        $DATA_DIR/bin/bash $DATA_DIR/scripts/proot-distro.sh login alpine -- \
        /bin/sh -c \"echo INSIDE_ALPINE; uname -a; cat /etc/os-release | head -3\"; \
        echo LOGIN_TEST_END'" 2>&1)

echo "$login_output" | grep -q "LOGIN_TEST_START" && log_pass "login command started" || log_fail "login command failed to start"
echo "$login_output" | grep -q "INSIDE_ALPINE" && log_pass "shell inside Alpine works" || log_fail "shell inside Alpine failed"
echo "$login_output" | grep -q "Alpine" && log_pass "Alpine /etc/os-release confirmed" || log_fail "Alpine identity not confirmed"

log_info "Login output:"
echo "$login_output" | sed 's/^/       /'

# ---- Step 4: Run apk update && apk add vim ----
echo ""
echo "=== Step 4: apk update && apk add vim ==="

apk_output=$(adb shell "run-as $PKG \
    env -i \
    APP_PREFIX=$DATA_DIR \
    APP_HOME=$HOME_DIR \
    APP_PACKAGE=$PKG \
    PATH=$DATA_DIR/bin:/system/bin:/system/xbin \
    HOME=$HOME_DIR \
    PROOT_NO_SECCOMP=1 \
    TERM=xterm-256color \
    LANG=en_US.UTF-8 \
    $DATA_DIR/bin/bash -c '$DATA_DIR/bin/bash $DATA_DIR/scripts/proot-distro.sh login alpine -- \
        /bin/sh -c \"apk update && apk add vim && vim --version | head -1\"'" 2>&1)

echo "$apk_output" | grep -q "apk update" && log_pass "apk update ran" || log_fail "apk update failed"
echo "$apk_output" | grep -q "Installing vim" && log_pass "vim installed" || log_fail "vim install failed"
echo "$apk_output" | grep -q "VIM - Vi IMproved" && log_pass "vim --version confirmed" || log_fail "vim --version check failed"

log_info "apk output:"
echo "$apk_output" | sed 's/^/       /'

# ---- Step 5: Remove Alpine ----
echo ""
echo "=== Step 5: Remove Alpine ==="

remove_output=$(adb shell "run-as $PKG \
    env -i \
    APP_PREFIX=$DATA_DIR \
    APP_HOME=$HOME_DIR \
    APP_PACKAGE=$PKG \
    PATH=$DATA_DIR/bin:/system/bin:/system/xbin \
    HOME=$HOME_DIR \
    PROOT_NO_SECCOMP=1 \
    TERM=xterm-256color \
    LANG=en_US.UTF-8 \
    $DATA_DIR/bin/bash $DATA_DIR/scripts/proot-distro.sh remove alpine" 2>&1)

echo "$remove_output" | sed 's/^/       /'
echo ""

if adb shell "run-as $PKG test -d $DATA_DIR/var/lib/proot-distro/installed-rootfs/alpine && echo EXISTS" 2>/dev/null | grep -q "EXISTS"; then
    log_fail "Alpine rootfs still exists after remove"
else
    log_pass "Alpine rootfs removed"
fi

# ---- Summary ----
echo ""
echo "========================================="
echo "  T5.2 Alpine Integration Test Results"
echo "========================================="
echo "  ${GREEN}PASS${NC}: $pass"
echo "  ${RED}FAIL${NC}: $fail"
echo "  ${YELLOW}SKIP${NC}: $skip"
echo "========================================="

if [ "$fail" -gt 0 ]; then
    echo ""
    echo "Check logcat for errors:"
    echo "  adb logcat -d -s PR:V AndroidRuntime:E"
    exit 1
fi

exit 0
