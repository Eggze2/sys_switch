#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ICON_SOURCE="$PROJECT_DIR/assets/sys-switch-icon.png"
ICON_OUTPUT_BASE="$PROJECT_DIR/build/icons"

if ! command -v convert &>/dev/null; then
    echo "Error: ImageMagick (convert) is required. Install with: sudo apt install imagemagick"
    exit 1
fi

if ! command -v cargo-deb &>/dev/null; then
    echo "Installing cargo-deb..."
    cargo install cargo-deb
fi

echo "Resizing icons..."
for SIZE in 48 128 256; do
    OUT_DIR="$ICON_OUTPUT_BASE/${SIZE}x${SIZE}"
    mkdir -p "$OUT_DIR"
    convert "$ICON_SOURCE" -resize "${SIZE}x${SIZE}" "$OUT_DIR/sys-switch.png"
    echo "  Created ${SIZE}x${SIZE} icon"
done

echo "Building release binary..."
cargo build --release --manifest-path "$PROJECT_DIR/Cargo.toml"

echo "Building .deb package..."
cargo deb --manifest-path "$PROJECT_DIR/Cargo.toml"

echo "Done: $PROJECT_DIR/target/debian/sys-switch_"*.deb
