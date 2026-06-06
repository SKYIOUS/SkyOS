# run.ps1 - Run Sarga OS in QEMU (Windows)
# initrd is embedded into the kernel — only ONE drive needed.

$KERNEL_PATH = "..\SKYIOUS KERNEL\target\x86_64-vahi\debug\bootimage-vahi_kernel.bin"

if (-not (Test-Path $KERNEL_PATH)) {
    Write-Error "Vahi Kernel bootimage not found at $KERNEL_PATH. Run 'python scripts/make_sarga_image.py' first."
    exit 1
}

Write-Host "Starting Sarga OS in QEMU..." -ForegroundColor Cyan

qemu-system-x86_64 `
  -bios "..\SKYIOUS KERNEL\OVMF.fd" `
  -drive "if=ide,format=raw,file=$KERNEL_PATH" `
  -m 512M `
  -smp 2 `
  -serial stdio `
  -vga std `
  -cpu max `
  -no-reboot
