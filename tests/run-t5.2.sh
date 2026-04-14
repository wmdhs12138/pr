#!/bin/sh
# Host-side: install APK, verify bootstrap, launch app for manual testing
set -e

APK="android/app/build/outputs/apk/debug/app-debug.apk"
PKG="id.or.oo.pr"

if [ ! -f "$APK" ]; then
    echo "APK not found. Run: cd android && ANDROID_HOME=/home/o/Android/Sdk ./gradlew assembleDebug"
    exit 1
fi

echo "=== Installing APK ==="
adb uninstall $PKG 2>/dev/null || true
adb install "$APK" 2>&1 | tail -1

echo "=== Launching app ==="
adb shell am start -n "$PKG/.MainActivity" > /dev/null 2>&1
sleep 4

echo "=== Verifying bootstrap ==="
adb push tests/t5.2-alpine.sh /data/local/tmp/t5.2-alpine.sh > /dev/null 2>&1
adb shell "run-as $PKG sh /data/local/tmp/t5.2-alpine.sh" 2>&1

echo ""
echo "========================================="
echo "  Bootstrap checks done."
echo "  App UI is open — proceed with manual tests:"
echo ""
echo "  1. Find Alpine Linux in the list"
echo "  2. Tap download icon to install"
echo "  3. Wait for install to complete"
echo "  4. Tap play icon to login"
echo "  5. In terminal run: uname -a"
echo "  6. In terminal run: apk update && apk add vim"
echo "  7. Press back to exit terminal"
echo "  8. Tap delete icon to remove"
echo "========================================="
