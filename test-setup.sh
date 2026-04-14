#!/system/bin/sh
# Setup test environment for proot-distro.sh on Android device
#
# Prerequisites (run from project root on host):
#   1. Build proot:        ./build.sh
#   2. Extract busybox:    tar xzf build/test-binaries/busybox-static.apk -C build/test-binaries/extracted/
#   3. Push everything:    adb push build/out/arm64/proot /data/local/tmp/
#                          adb push build/test-binaries/extracted/bin/busybox.static /data/local/tmp/
#                          adb push build/test-binaries/bash-static /data/local/tmp/
#                          adb push src/scripts/proot-distro.sh /data/local/tmp/
#                          adb push src/scripts/plugins /data/local/tmp/plugins
#                          adb push build/test-setup.sh /data/local/tmp/
#   4. Run setup:          adb shell sh /data/local/tmp/test-setup.sh
#
# After setup, test with:
#   adb shell
#   export PATH=/data/local/tmp/pr-test/usr/bin
#   export APP_PREFIX=/data/local/tmp/pr-test/usr
#   export APP_HOME=/data/local/tmp/pr-test/home
#   export APP_PACKAGE=id.or.oo.pr
#   export PROOT_NO_SECCOMP=1
#   proot-distro list

set -e

PREFIX="/data/local/tmp/pr-test/usr"
TEST_HOME="/data/local/tmp/pr-test/home"
BB="/data/local/tmp/busybox.static"

echo "=== Setting up test environment ==="
echo "PREFIX=${PREFIX}"

# Create directory structure
mkdir -p "${PREFIX}/bin"
mkdir -p "${PREFIX}/etc/proot-distro"
mkdir -p "${PREFIX}/var/lib/proot-distro/installed-rootfs"
mkdir -p "${PREFIX}/var/lib/proot-distro/dlcache"
mkdir -p "${TEST_HOME}"
mkdir -p "${PREFIX}/tmp"

# Install busybox-static
cp "${BB}" "${PREFIX}/bin/busybox"
chmod 755 "${PREFIX}/bin/busybox"

# Install static bash
cp /data/local/tmp/bash-static "${PREFIX}/bin/bash"
chmod 755 "${PREFIX}/bin/bash"

# Install proot binary
cp /data/local/tmp/proot "${PREFIX}/bin/proot"
chmod 755 "${PREFIX}/bin/proot"

# Create busybox applet symlinks
cd "${PREFIX}/bin"
for applet in awk basename cat chmod cp cut du file find grep gzip \
    head id mkdir mv realpath rm sed stat tar touch tr xargs sleep \
    printf dd dirname sort wc date sha256sum wget uname \
    expr test echo readlink; do
    [ -L "$applet" ] && rm "$applet"
    [ -e "$applet" ] && continue
    ln -s busybox "$applet"
done
cd /data/local/tmp

echo "Busybox applets: $(ls ${PREFIX}/bin | wc -l)"

# Copy proot-distro.sh
cp /data/local/tmp/proot-distro.sh "${PREFIX}/bin/proot-distro"
chmod 755 "${PREFIX}/bin/proot-distro"

# Copy plugins
if [ -d /data/local/tmp/plugins ]; then
    for f in /data/local/tmp/plugins/*.sh; do
        [ -f "$f" ] && cp "$f" "${PREFIX}/etc/proot-distro/"
    done
fi

echo ""
echo "=== Setup complete ==="
echo "Binaries: $(ls ${PREFIX}/bin | wc -l)"
echo "Plugins: $(ls ${PREFIX}/etc/proot-distro | wc -l)"
echo ""
echo "To test:"
echo "  export PATH=${PREFIX}/bin"
echo "  export APP_PREFIX=${PREFIX}"
echo "  export APP_HOME=${TEST_HOME}"
echo "  export APP_PACKAGE=id.or.oo.pr"
echo "  export PROOT_NO_SECCOMP=1"
echo "  proot-distro list"
