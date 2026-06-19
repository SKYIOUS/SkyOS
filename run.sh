#!/bin/bash
# run.sh - Run Sarga OS in QEMU (WSL/Linux)
# initrd is embedded into the kernel — only ONE drive needed.

KERNEL_PATH="../SKYIOUS KERNEL/target/x86_64-vahi/debug/bootimage-vahi_kernel.bin"

if [ ! -f "$KERNEL_PATH" ]; then
    echo "Error: SARGA Kernel bootimage not found at $KERNEL_PATH"
    echo "Run: python scripts/make_sarga_image.py"
    exit 1
fi

echo "Starting Sarga OS in QEMU..."

qemu-system-x86_64 \
  -drive if=ide,format=raw,file="$KERNEL_PATH" \
  -m 512M -smp 2 -serial stdio \
  -vga std -cpu max -no-reboot
