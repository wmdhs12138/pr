#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
JNI_DIR="$PROJECT_ROOT/android/app/src/main/jniLibs/arm64-v8a"

echo "Building pr-cli for aarch64-linux-android..."
cd "$SCRIPT_DIR"
cargo build --target aarch64-linux-android --release

BINARY="$SCRIPT_DIR/target/aarch64-linux-android/release/pr-cli"
if [ ! -f "$BINARY" ]; then
    echo "Error: binary not found at $BINARY"
    exit 1
fi

SIZE=$(stat -c%s "$BINARY")
echo "Binary size: $(( SIZE / 1024 ))KB"

mkdir -p "$JNI_DIR"
cp "$BINARY" "$JNI_DIR/libpr-cli.so"
echo "Copied to $JNI_DIR/libpr-cli.so"
