#!/usr/bin/env bash
# QEMU integration test for SkyOS
# Builds everything, boots in QEMU, logs in, runs commands, checks output.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SKYOS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
KERNEL_DIR="${1:-$SKYOS_DIR/../SKYIOUS KERNEL}"
TIMEOUT=180

echo "=== SkyOS QEMU Integration Test ==="
echo "Kernel dir: $KERNEL_DIR"
echo "SkyOS dir:  $SKYOS_DIR"

# Build kernel
echo "--- Building kernel ---"
cd "$KERNEL_DIR/kernel"
cargo build --release --target x86_64-unknown-none \
    -Zbuild-std=core,alloc --features net,smp,ai_rule

# Build userspace
echo "--- Building userspace ---"
cd "$SKYOS_DIR"
cargo build -Zbuild-std=core,alloc --target x86_64-sarga.json --release

# Build initrd
echo "--- Building initrd ---"
cd "$SKYOS_DIR"
python3 build_initrd.py
mkdir -p "$KERNEL_DIR/SkyOS"
cp initrd.tar "$KERNEL_DIR/SkyOS/"

# Create UEFI bootimage
echo "--- Creating UEFI bootimage ---"
cd "$KERNEL_DIR"
cargo run --release --manifest-path builder/Cargo.toml

# Create ISO
echo "--- Creating ISO ---"
cd "$SKYOS_DIR"
pip install pycdlib 2>/dev/null || true
python3 scripts/make_iso.py "integration-$(date +%Y%m%d-%H%M%S)"

# Find ISO
ISO=$(ls "$SKYOS_DIR/release/skyos-integration-"*.iso 2>/dev/null | head -1)
if [ -z "$ISO" ]; then
    echo "ERROR: No ISO found!"
    exit 1
fi
echo "ISO: $ISO"

# Run QEMU with expect for automated interaction
echo "--- Running QEMU integration test ---"
cd "$SKYOS_DIR"

if command -v expect &>/dev/null; then
    expect "$SCRIPT_DIR/qemu_shell_test.exp" "$ISO" "$TIMEOUT"
    RESULT=$?
else
    echo "WARNING: expect not found; falling back to simple boot test"
    timeout "$TIMEOUT" qemu-system-x86_64 \
        -bios OVMF.fd \
        -cdrom "$ISO" \
        -m 512M -smp 2 \
        -nographic -no-reboot \
        -serial mon:stdio \
        -device e1000,netdev=net0 -netdev user,id=net0 \
        2>&1 | tee qemu_integration_log.txt

    if grep -q "login:" qemu_integration_log.txt; then
        echo "PASS: System booted to login prompt"
        RESULT=0
    else
        echo "FAIL: No login prompt found"
        grep -i "panic\|error\|fail" qemu_integration_log.txt || true
        RESULT=1
    fi
fi

if [ "$RESULT" -eq 0 ]; then
    echo "============================================"
    echo "QEMU INTEGRATION TEST: PASS"
    echo "============================================"
else
    echo "============================================"
    echo "QEMU INTEGRATION TEST: FAIL"
    echo "============================================"
fi
exit $RESULT
