#!/bin/bash
# build_installer_iso.sh — Build a bootable SkyOS installer ISO (Linux/WSL)
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
KERNEL_DIR="$REPO_DIR/../SKYIOUS KERNEL"
OUTPUT_ISO="$REPO_DIR/skyos-installer.iso"
OUTPUT_IMG="$REPO_DIR/skyos-installer.img"

echo "=== SkyOS Installer ISO Builder ==="

# Build bootimage
if [ "$1" = "--full" ]; then
    echo "[1/3] Building userspace..."
    cd "$KERNEL_DIR"
    pwsh -ExecutionPolicy Bypass -File build_userspace.ps1
    echo "[2/3] Building kernel + bootimage..."
    cargo build --target x86_64-unknown-none --manifest-path kernel/Cargo.toml
    cargo run --manifest-path builder/Cargo.toml
fi

BOOTIMG="$KERNEL_DIR/target/x86_64-vahi/debug/bootimage-vahi_kernel.bin"
if [ ! -f "$BOOTIMG" ]; then
    echo "Bootimage not found. Run with --full or build manually."
    exit 1
fi
echo "[OK] Bootimage: $BOOTIMG ($(stat -f%z "$BOOTIMG" 2>/dev/null || wc -c < "$BOOTIMG") bytes)"

# Create ISO
echo "[3/3] Creating ISO..."
EXTRA_DIR=$(mktemp -d)
echo "SkyOS Installation Media" > "$EXTRA_DIR/README.TXT"

if command -v xorriso &> /dev/null; then
    xorriso -as mkisofs -o "$OUTPUT_ISO" -V "SKYOS_INSTALL" \
        -e "$BOOTIMG" -no-emul-boot -boot-load-size 4 -isohybrid-gpt-basdat "$EXTRA_DIR"
    echo "[OK] ISO: $OUTPUT_ISO"
elif command -v mkisofs &> /dev/null; then
    mkisofs -o "$OUTPUT_ISO" -V "SKYOS_INSTALL" -e "$BOOTIMG" -no-emul-boot "$EXTRA_DIR"
    echo "[OK] ISO: $OUTPUT_ISO"
else
    echo "No ISO tool found. Creating raw disk image..."
    IMG_SIZE=64
    dd if=/dev/zero of="$OUTPUT_IMG" bs=1M count=$IMG_SIZE 2>/dev/null
    dd if="$BOOTIMG" of="$OUTPUT_IMG" conv=notrunc 2>/dev/null
    echo "[OK] IMG: $OUTPUT_IMG (${IMG_SIZE} MB)"
fi

rm -rf "$EXTRA_DIR"
echo "Done."
echo "  QEMU: qemu-system-x86_64 -cdrom $OUTPUT_ISO -m 512M"
