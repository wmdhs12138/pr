#!/bin/bash
# Push test environment to device using bootstrap.sh
#
# Usage:
#   scripts/test-push.sh              # Push files only
#   scripts/test-push.sh setup        # Push + run bootstrap
#   scripts/test-push.sh test         # Push + bootstrap + proot-distro list
#   scripts/test-push.sh shell        # Push + bootstrap + interactive shell
#
# Prerequisites:
#   - adb in PATH, device connected
#   - proot built:     scripts/build.sh --arch=arm64
#   - busybox ready:   scripts/download-busybox.sh
#   - bash ready:      scripts/download-bash.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
PROOT="${PROJECT_ROOT}/build/out/arm64/proot"
BB="${PROJECT_ROOT}/build/assets/arm64-v8a/busybox"
BASH="${PROJECT_ROOT}/build/assets/arm64-v8a/bash"
PDISTRO="${PROJECT_ROOT}/src/scripts/proot-distro.sh"
BOOTSTRAP="${PROJECT_ROOT}/src/scripts/bootstrap.sh"
PLUGINS="${PROJECT_ROOT}/src/scripts/plugins"

DEVICE_PREFIX="/data/local/tmp/pr-test/usr"

check_file() {
    if [ ! -f "$1" ]; then
        echo "ERROR: Missing $2: $1"
        echo "  $3"
        exit 1
    fi
}

check_file "$PROOT" "proot binary" "Run: scripts/build.sh --arch=arm64"
check_file "$BB" "busybox" "Run: scripts/download-busybox.sh"
check_file "$BASH" "bash" "Run: scripts/download-bash.sh"
check_file "$PDISTRO" "proot-distro.sh" ""
check_file "$BOOTSTRAP" "bootstrap.sh" ""

echo "=== Pushing files to device ==="

echo "Pushing binaries..."
adb push "$PROOT" "${DEVICE_PREFIX}/bin/proot"
adb push "$BB" "${DEVICE_PREFIX}/bin/busybox"
adb push "$BASH" "${DEVICE_PREFIX}/bin/bash"

echo "Pushing scripts..."
adb push "$BOOTSTRAP" "${DEVICE_PREFIX}/bin/bootstrap.sh"
adb push "$PDISTRO" "${DEVICE_PREFIX}/scripts/proot-distro.sh"

echo "Pushing plugins..."
adb shell rm -rf "${DEVICE_PREFIX}/plugins"
adb push "$PLUGINS" "${DEVICE_PREFIX}/plugins"

echo ""
echo "=== Push complete ==="

ACTION="${1:-}"

ENV_EXPORTS="export PATH=${DEVICE_PREFIX}/bin APP_PREFIX=${DEVICE_PREFIX} APP_HOME=/data/local/tmp/pr-test/home APP_PACKAGE=id.or.oo.pr PROOT_NO_SECCOMP=1"

case "$ACTION" in
    setup)
        echo ""
        echo "=== Running bootstrap ==="
        adb shell "rm -f ${DEVICE_PREFIX}/.bootstrapped"
        adb shell "APP_PREFIX=${DEVICE_PREFIX} APP_HOME=/data/local/tmp/pr-test/home APP_PACKAGE=id.or.oo.pr sh ${DEVICE_PREFIX}/bin/bootstrap.sh"
        ;;
    test)
        echo ""
        echo "=== Running bootstrap ==="
        adb shell "rm -f ${DEVICE_PREFIX}/.bootstrapped"
        adb shell "APP_PREFIX=${DEVICE_PREFIX} APP_HOME=/data/local/tmp/pr-test/home APP_PACKAGE=id.or.oo.pr sh ${DEVICE_PREFIX}/bin/bootstrap.sh"
        echo ""
        echo "=== Running proot-distro list ==="
        adb shell "${ENV_EXPORTS} && proot-distro list"
        ;;
    shell)
        echo ""
        echo "=== Running bootstrap ==="
        adb shell "rm -f ${DEVICE_PREFIX}/.bootstrapped"
        adb shell "APP_PREFIX=${DEVICE_PREFIX} APP_HOME=/data/local/tmp/pr-test/home APP_PACKAGE=id.or.oo.pr sh ${DEVICE_PREFIX}/bin/bootstrap.sh"
        echo ""
        echo "=== Starting interactive shell ==="
        adb shell "${ENV_EXPORTS} && bash"
        ;;
    *)
        echo ""
        echo "To continue:"
        echo "  adb shell 'rm -f ${DEVICE_PREFIX}/.bootstrapped && APP_PREFIX=${DEVICE_PREFIX} sh ${DEVICE_PREFIX}/bin/bootstrap.sh'"
        echo ""
        echo "Then test:"
        echo "  adb shell"
        echo "  ${ENV_EXPORTS}"
        echo "  proot-distro list"
        ;;
esac
