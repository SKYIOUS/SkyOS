$ErrorActionPreference = "Stop"
$scriptDir = $PSScriptRoot
$KERNEL_PATH = "$scriptDir\kernel\target\x86_64-vahi\debug\bootimage-vahi_kernel.bin"
$BIOS_PATH = "$scriptDir\OVMF.fd"
$LOG_PATH = "$scriptDir\qemu_display.log"

Write-Host "Starting SkyOS in QEMU (SDL display)..." -ForegroundColor Cyan
Write-Host "Boot log will be written to: $LOG_PATH" -ForegroundColor Gray
Write-Host "Press Ctrl+Alt+G to release mouse/keyboard grab." -ForegroundColor Gray
Write-Host "IMPORTANT: Do NOT add -usb -device usb-tablet - kernel only has PS/2 mouse driver" -ForegroundColor Yellow

Remove-Item $LOG_PATH -ErrorAction SilentlyContinue

$qemuArgs = @(
  "-bios", $BIOS_PATH,
  "-drive", "if=ide,format=raw,file=$KERNEL_PATH",
  "-m", "512M",
  "-smp", "1",
  "-vga", "std",
  "-cpu", "max",
  "-no-reboot",
  "-k", "en-us",
  "-display", "sdl",
  "-serial", "file:$LOG_PATH"
)

Start-Process -NoNewWindow -PassThru -FilePath "qemu-system-x86_64" -ArgumentList $qemuArgs
