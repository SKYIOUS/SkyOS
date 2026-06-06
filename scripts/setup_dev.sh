#!/bin/bash
# setup_dev.sh — One-click SkyOS development environment setup (Linux/WSL)
set -e

echo "=== SkyOS Dev Environment Setup ==="

# 1. Check Rust
if ! command -v rustc &> /dev/null; then
    echo "[INSTALL] Rust not found. Install from https://rustup.rs first."
    exit 1
fi
echo "[OK] Rust: $(rustc --version)"

# 2. Install nightly + components
echo "[SETUP] Installing nightly toolchain..."
rustup toolchain install nightly --allow-downgrade -c rust-src -c llvm-tools-preview 2>/dev/null
echo "[OK] nightly toolchain ready"

# 3. Install targets
for t in x86_64-unknown-none x86_64-skyos; do
    rustup target add "$t" --toolchain nightly 2>/dev/null
    echo "[OK] target $t"
done

# 4. Check QEMU
if command -v qemu-system-x86_64 &> /dev/null; then
    echo "[OK] QEMU: $(which qemu-system-x86_64)"
else
    echo "[WARN] QEMU not found. Install: sudo apt install qemu-system-x86-64"
fi

# 5. Check Python
if command -v python3 &> /dev/null; then
    echo "[OK] Python3: $(python3 --version)"
elif command -v python &> /dev/null; then
    echo "[OK] Python: $(python --version)"
else
    echo "[WARN] Python not found"
fi

# 6. Install bootimage
if ! command -v cargo-bootimage &> /dev/null; then
    echo "[SETUP] Installing cargo-bootimage..."
    cargo install bootimage 2>/dev/null
fi
echo "[OK] bootimage ready"

echo ""
echo "=== Dev environment ready ==="
echo "  Build:          ./build.sh"
echo "  Boot image:     cd ../SKYIOUS\ KERNEL; ./make_bootimage.sh"
echo "  Run:            ./run.sh"
