#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="${SCRIPT_DIR}/build"

# robxu9/bash-static release details
# Source: https://github.com/robxu9/bash-static/releases
BASH_VERSION="5.2.015"
BASH_TAG="5.2.015-1.2.3-2"
BASH_RELEASE_TAG="${BASH_TAG}"

BASH_FILE="bash-linux-aarch64"
BASH_URL="https://github.com/robxu9/bash-static/releases/download/${BASH_RELEASE_TAG}/${BASH_FILE}"
BASH_SHA256="8877ad33344af461ed801066322fd9a7808cd73e4e81087da228e32e8fad54ca"

# Output paths
DL_DIR="${BUILD_DIR}/dl"
OUT_DIR="${BUILD_DIR}/assets/arm64-v8a"
OUT_FILE="${OUT_DIR}/bash"

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Download and prepare static bash binary from robxu9/bash-static."
    echo ""
    echo "Options:"
    echo "  -f, --force       Re-download even if binary exists"
    echo "  --verify-only     Only verify existing binary, skip download"
    echo "  -h, --help        Show this help"
    echo ""
    echo "Output: ${OUT_FILE}"
}

die() {
    echo "ERROR: $*" >&2
    exit 1
}

info() {
    echo "==> $*"
}

verify_binary_file() {
    local binary="$1"

    info "Verifying: $binary"

    [ -f "$binary" ] || die "Binary not found: $binary"

    info "file(1) output:"
    file "$binary"

    local actual_class
    actual_class=$(readelf -h "$binary" 2>/dev/null | grep "Class:" | awk '{print $NF}')
    local actual_machine
    actual_machine=$(readelf -h "$binary" 2>/dev/null | grep "Machine:" | head -1)

    if [ "$actual_class" != "ELF64" ]; then
        die "Expected ELF64, got: $actual_class"
    fi

    if echo "$actual_machine" | grep -qi "aarch"; then
        :
    else
        die "Expected AArch64, got: $actual_machine"
    fi

    if readelf -d "$binary" 2>/dev/null | grep -q "NEEDED"; then
        die "Binary has dynamic dependencies (not static!)"
    fi
    info "OK: Static binary confirmed"

    local tls_align_hex
    tls_align_hex=$(readelf -W -l "$binary" 2>/dev/null \
        | awk '/^  TLS/{print $NF}' \
        | sed 's/0x//')

    if [ -n "$tls_align_hex" ]; then
        local tls_align=$((16#$tls_align_hex))
        if [ "$tls_align" -lt 64 ]; then
            echo "WARNING: TLS alignment is ${tls_align} bytes (need >= 64 for Android Bionic)"
            echo "  Will need post-build fix"
        else
            info "TLS alignment: ${tls_align} bytes (OK)"
        fi
    else
        info "No PT_TLS segment (OK for static binary)"
    fi

    info "Size: $(du -h "$binary" | cut -f1)"
}

verify_sha256() {
    local file="$1"
    local expected="$2"
    local actual
    actual=$(sha256sum "$file" | awk '{print $1}')
    if [ "$actual" != "$expected" ]; then
        die "SHA256 mismatch for $(basename "$file")\n  Expected: $expected\n  Actual:   $actual"
    fi
    info "SHA256 OK: $(basename "$file")"
}

main() {
    local force=false
    local verify_only=false

    for arg in "$@"; do
        case "$arg" in
            -f|--force)
                force=true
                ;;
            --verify-only)
                verify_only=true
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                die "Unknown option: $arg"
                ;;
        esac
    done

    if $verify_only; then
        if [ -f "$OUT_FILE" ]; then
            verify_sha256 "$OUT_FILE" "$BASH_SHA256"
            verify_binary_file "$OUT_FILE"
            info "Verification complete"
        else
            die "Binary not found: $OUT_FILE"
        fi
        exit 0
    fi

    if [ -f "$OUT_FILE" ] && ! $force; then
        info "Binary already exists: $OUT_FILE"
        verify_sha256 "$OUT_FILE" "$BASH_SHA256"
        info "Use --force to re-download"
        exit 0
    fi

    mkdir -p "$DL_DIR" "$OUT_DIR"

    local dl_path="${DL_DIR}/${BASH_FILE}"

    if [ -f "$dl_path" ] && ! $force; then
        info "Using cached binary: $dl_path"
    else
        info "Downloading bash-static ${BASH_VERSION} ..."
        info "URL: ${BASH_URL}"
        curl -fSL -o "$dl_path" "$BASH_URL" || die "Download failed"
    fi

    verify_sha256 "$dl_path" "$BASH_SHA256"

    cp "$dl_path" "$OUT_FILE"
    chmod 755 "$OUT_FILE"

    verify_binary_file "$OUT_FILE"

    echo ""
    info "Static bash ready: ${OUT_FILE}"
    info "  Version:  GNU bash ${BASH_VERSION}"
    info "  Source:   robxu9/bash-static (${BASH_RELEASE_TAG})"
    info "  License:  GPL-3.0-or-later"
}

main "$@"
