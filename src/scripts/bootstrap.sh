#!/system/bin/sh
# bootstrap.sh — First-run setup for standalone proot-distro on Android
#
# This script MUST be POSIX sh compatible (#!/system/bin/sh).
# Keep this script POSIX-compatible; avoid bash-isms.
#
# Called by APK BootstrapService on first launch:
#   /system/bin sh /data/data/id.or.oo.pr/files/usr/bin/bootstrap.sh
#
# Or for testing via adb:
#   adb push src/scripts/bootstrap.sh /data/local/tmp/
#   adb shell sh /data/local/tmp/bootstrap.sh
#
# Environment variables (set by caller):
#   APP_PREFIX  - Base directory (default: /data/data/id.or.oo.pr/files/usr)
#   APP_HOME    - Home directory (default: /data/data/id.or.oo.pr/files/home)
#   APP_PACKAGE - Android package (default: id.or.oo.pr)

set -e

APP_PREFIX="${APP_PREFIX:-/data/data/id.or.oo.pr/files/usr}"
APP_HOME="${APP_HOME:-/data/data/id.or.oo.pr/files/home}"
APP_PACKAGE="${APP_PACKAGE:-id.or.oo.pr}"

BIN_DIR="${APP_PREFIX}/bin"
ETC_DIR="${APP_PREFIX}/etc/proot-distro"
VAR_DIR="${APP_PREFIX}/var/lib/proot-distro"

MARKER="${APP_PREFIX}/.bootstrapped"

log() {
    echo "[bootstrap] $*"
}

die() {
    echo "[bootstrap] ERROR: $*" >&2
    exit 1
}

check_already_bootstrapped() {
    if [ -f "$MARKER" ]; then
        log "Already bootstrapped (remove ${MARKER} to re-run)"
        exit 0
    fi
}

create_directories() {
    log "Creating directory structure..."
    mkdir -p "${BIN_DIR}"
    mkdir -p "${ETC_DIR}"
    mkdir -p "${VAR_DIR}/installed-rootfs"
    mkdir -p "${VAR_DIR}/dlcache"
    mkdir -p "${APP_HOME}"
    mkdir -p "${APP_PREFIX}/tmp"
    mkdir -p "${APP_PREFIX}/scripts"
}

install_busybox() {
    log "Installing busybox..."
    local bb_src="${BIN_DIR}/busybox"
    if [ ! -e "$bb_src" ]; then
        die "busybox not found at ${bb_src}"
    fi
    [ -f "$bb_src" ] && chmod 755 "$bb_src" 2>/dev/null || true

    if [ -L "${BIN_DIR}/sh" ]; then
        log "Applet symlinks already exist, skipping creation"
    else
        log "Creating applet symlinks..."
        ln -s busybox "${BIN_DIR}/sh" 2>/dev/null || true
        for applet in awk basename cat chmod cp curl cut date dirname grep head \
            id ln ls mkdir mv printf pwd readlink realpath rm rmdir sed stat \
            tail tar test tr uname uniq wc wget which xargs; do
            [ -L "${BIN_DIR}/${applet}" ] && continue
            [ -e "${BIN_DIR}/${applet}" ] && continue
            ln -s busybox "${BIN_DIR}/${applet}" 2>/dev/null || true
        done
    fi

    local count
    count=$(ls "${BIN_DIR}" | wc -l)
    log "busybox: ${count} entries in ${BIN_DIR}"
}

install_proot() {
    log "Installing proot..."
    local proot_src="${BIN_DIR}/proot"
    if [ ! -e "$proot_src" ]; then
        die "proot not found at ${proot_src}"
    fi
    [ -f "$proot_src" ] && chmod 755 "$proot_src" 2>/dev/null || true
    log "proot installed"
}

install_proot_distro() {
    log "Installing proot-distro.sh..."
    local src="${APP_PREFIX}/scripts/proot-distro.sh"
    local dst="${BIN_DIR}/proot-distro"
    if [ ! -f "$src" ]; then
        die "proot-distro.sh not found at ${src}"
    fi

    cp "$src" "$dst"
    chmod 755 "$dst"

    if grep -q '@APP_PREFIX@' "$dst" 2>/dev/null; then
        log "Replacing @APP_PREFIX@ template in shebang..."
        sed -i "s|@APP_PREFIX@|${APP_PREFIX}|g" "$dst"
    fi

    log "proot-distro installed"
}

install_plugins() {
    log "Installing distro plugins..."
    local plugin_dir="${APP_PREFIX}/plugins"
    if [ ! -d "$plugin_dir" ]; then
        log "No plugins directory at ${plugin_dir}, skipping"
        return
    fi

    local count=0
    local f
    for f in "${plugin_dir}"/*.sh; do
        [ -f "$f" ] || continue
        cp "$f" "${ETC_DIR}/"
        count=$((count + 1))
    done

    log "${count} plugins installed"
}

mark_bootstrapped() {
    local ts
    ts=$(date +%s 2>/dev/null || echo "0")
    echo "${ts}" > "$MARKER"
    log "Bootstrap complete (marker: ${MARKER})"
}

print_summary() {
    echo ""
    echo "=== Bootstrap Summary ==="
    echo "  APP_PREFIX:  ${APP_PREFIX}"
    echo "  APP_HOME:    ${APP_HOME}"
    echo "  APP_PACKAGE: ${APP_PACKAGE}"
    echo ""
    echo "  Binaries:    $(ls "${BIN_DIR}" | wc -l)"
    echo "  Plugins:     $(ls "${ETC_DIR}"/*.sh 2>/dev/null | wc -l)"
    echo ""
    echo "  proot:       $([ -x "${BIN_DIR}/proot" ] && echo "OK" || echo "MISSING")"
    echo "  busybox:     $([ -x "${BIN_DIR}/busybox" ] && echo "OK" || echo "MISSING")"
    echo "  proot-distro:$([ -x "${BIN_DIR}/proot-distro" ] && echo "OK" || echo "MISSING")"
    echo ""
    echo "To test:"
    echo "  export PATH=${BIN_DIR}"
    echo "  export APP_PREFIX=${APP_PREFIX}"
    echo "  export APP_HOME=${APP_HOME}"
    echo "  export APP_PACKAGE=${APP_PACKAGE}"
    echo "  export PROOT_NO_SECCOMP=1"
    echo "  proot-distro list"
}

main() {
    log "Starting bootstrap for ${APP_PACKAGE}"
    log "APP_PREFIX=${APP_PREFIX}"

    check_already_bootstrapped
    create_directories
    install_busybox
    install_proot
    install_proot_distro
    install_plugins
    mark_bootstrapped
    print_summary
}

main "$@"
