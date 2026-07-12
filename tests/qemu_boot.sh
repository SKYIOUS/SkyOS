#!/usr/bin/env bash
set -euo pipefail

# QEMU boot smoke test for SkyOS
# Usage: ./tests/qemu_boot.sh [kernel_dir]
# Assumes SKYIOUS-KERNEL is a sibling directory by default.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SKYOS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
KERNEL_DIR="${1:-$SKYOS_DIR/../SKYIOUS KERNEL}"

TIMEOUT=120
LOGFILE=$(mktemp)

cleanup() {
    rm -f "$LOGFILE"
}
trap cleanup EXIT

check_prereqs() {
    for cmd in qemu-system-x86_64 cargo python3 xorriso; do
        if ! command -v "$cmd" &>/dev/null; then
            echo "ERROR: $cmd not found. Install it first."
            exit 1
        fi
    done
}

build_kernel() {
    echo "=== Building kernel ==="
    cd "$KERNEL_DIR/kernel"
    cargo build --release --target x86_64-unknown-none \
        -Zbuild-std=core,alloc --features net,smp,ai_rule
}

build_userspace() {
    echo "=== Building userspace ==="
    cd "$SKYOS_DIR"
    cargo build -Zbuild-std=core,alloc --target x86_64-sarga.json --release
}

build_initrd() {
    echo "=== Building initrd ==="
    cd "$SKYOS_DIR"
    python3 build_initrd.py
    mkdir -p "$KERNEL_DIR/SkyOS"
    cp initrd.tar "$KERNEL_DIR/SkyOS/"
}

build_bootimage() {
    echo "=== Creating UEFI bootimage ==="
    cd "$KERNEL_DIR"
    cargo run --release --manifest-path builder/Cargo.toml
}

build_iso() {
    echo "=== Creating ISO ==="
    cd "$SKYOS_DIR"
    pip install pycdlib 2>/dev/null || true
    python3 scripts/make_iso.py "boottest-$(date +%Y%m%d)"
}

run_qemu() {
    echo "=== Booting in QEMU ==="
    cd "$SKYOS_DIR"
    ISO=$(ls release/skyos-boottest-*.iso 2>/dev/null | head -1)
    if [ -z "$ISO" ]; then
        echo "ERROR: No ISO found in release/"
        exit 1
    fi

    timeout "$TIMEOUT" qemu-system-x86_64 \
        -bios OVMF.fd \
        -cdrom "$ISO" \
        -m 512M -smp 2 \
        -nographic -no-reboot \
        -serial mon:stdio \
        -device e1000,netdev=net0 -netdev user,id=net0 \
        2>&1 | tee "$LOGFILE"
}

check_result() {
    echo "=== Checking boot log ==="
    if grep -q "login:" "$LOGFILE"; then
        echo "PASS: SkyOS booted successfully (login prompt detected)"
        return 0
    fi
    if grep -qi "panic" "$LOGFILE"; then
        echo "FAIL: Kernel panic detected in boot"
        return 1
    fi
    echo "FAIL: No login prompt found. Log output:"
    cat "$LOGFILE"
    return 1
}

# --- Main ---
check_prereqs
build_kernel
build_userspace
build_initrd
build_bootimage
build_iso
run_qemu
check_result
