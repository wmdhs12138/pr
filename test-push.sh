#!/bin/bash
# Push test environment to device and optionally run commands
#
# Usage:
#   ./build/test-push.sh              # Push all files
#   ./build/test-push.sh setup        # Push + run setup
#   ./build/test-push.sh test         # Push + setup + run tests
#   ./build/test-push.sh shell        # Push + setup + interactive shell
#
# Prerequisites:
#   - adb in PATH, device connected
#   - proot built (./build.sh)
#   - busybox-static extracted (tar xzf build/test-binaries/busybox-static.apk -C build/test-binaries/extracted/)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BB_EXTRACTED="${PROJECT_DIR}/build/test-binaries/extracted/bin/busybox.static"
BASH_STATIC="${PROJECT_DIR}/build/test-binaries/bash-static"
PROOT="${PROJECT_DIR}/build/out/arm64/proot"
PDISTRO="${PROJECT_DIR}/src/scripts/proot-distro.sh"
PLUGINS="${PROJECT_DIR}/src/scripts/plugins"
SETUP="${PROJECT_DIR}/build/test-setup.sh"

check_file() {
    if [ ! -f "$1" ]; then
        echo "ERROR: Missing $2: $1"
        echo "  $3"
        exit 1
    fi
}

check_file "$PROOT" "proot binary" "Run: ./build.sh"
check_file "$BB_EXTRACTED" "busybox-static" "Run: tar xzf build/test-binaries/busybox-static.apk -C build/test-binaries/extracted/"
check_file "$BASH_STATIC" "static bash" "Run: curl -fSL -o build/test-binaries/bash-static https://github.com/robxu9/bash-static/releases/download/5.2.015-1.2.3-2/bash-linux-aarch64"
check_file "$PDISTRO" "proot-distro.sh" ""
check_file "$SETUP" "test-setup.sh" ""

echo "=== Pushing files to device ==="

echo "Pushing proot..."
adb push "$PROOT" /data/local/tmp/proot

echo "Pushing busybox-static..."
adb push "$BB_EXTRACTED" /data/local/tmp/busybox.static

echo "Pushing bash-static..."
adb push "$BASH_STATIC" /data/local/tmp/bash-static

echo "Pushing proot-distro.sh..."
adb push "$PDISTRO" /data/local/tmp/proot-distro.sh

echo "Pushing plugins..."
adb shell rm -rf /data/local/tmp/plugins
adb push "$PLUGINS" /data/local/tmp/plugins

echo "Pushing test-setup.sh..."
adb push "$SETUP" /data/local/tmp/test-setup.sh

echo ""
echo "=== Push complete ==="

ACTION="${1:-}"

case "$ACTION" in
    setup)
        echo ""
        echo "=== Running setup on device ==="
        adb shell sh /data/local/tmp/test-setup.sh
        ;;
    test)
        echo ""
        echo "=== Running setup on device ==="
        adb shell sh /data/local/tmp/test-setup.sh
        echo ""
        echo "=== Running proot-distro list ==="
        adb shell "export PATH=/data/local/tmp/pr-test/usr/bin && export APP_PREFIX=/data/local/tmp/pr-test/usr && export APP_HOME=/data/local/tmp/pr-test/home && export APP_PACKAGE=id.or.oo.pr && export PROOT_NO_SECCOMP=1 && proot-distro list"
        ;;
    shell)
        echo ""
        echo "=== Running setup on device ==="
        adb shell sh /data/local/tmp/test-setup.sh
        echo ""
        echo "=== Starting interactive shell ==="
        echo "Run: proot-distro list"
        adb shell "export PATH=/data/local/tmp/pr-test/usr/bin && export APP_PREFIX=/data/local/tmp/pr-test/usr && export APP_HOME=/data/local/tmp/pr-test/home && export APP_PACKAGE=id.or.oo.pr && export PROOT_NO_SECCOMP=1 && bash"
        ;;
    *)
        echo ""
        echo "To continue:"
        echo "  adb shell sh /data/local/tmp/test-setup.sh"
        echo ""
        echo "Then test:"
        echo "  adb shell"
        echo "  export PATH=/data/local/tmp/pr-test/usr/bin"
        echo "  export APP_PREFIX=/data/local/tmp/pr-test/usr"
        echo "  export APP_HOME=/data/local/tmp/pr-test/home"
        echo "  export APP_PACKAGE=id.or.oo.pr"
        echo "  export PROOT_NO_SECCOMP=1"
        echo "  proot-distro list"
        ;;
esac
