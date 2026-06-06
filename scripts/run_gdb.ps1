# run_gdb.ps1 — Launch QEMU with GDB server and attach GDB for kernel debugging
param(
    [switch]$NoGdb,       # Start QEMU only, don't launch GDB
    [int]$Port = 1234,    # GDB server port
    [string]$KernelDir    # Override kernel dir
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $PSCommandPath
$kernelDir = if ($KernelDir) { $KernelDir } else { Resolve-Path "$scriptDir\..\..\SKYIOUS KERNEL" }
$bootimg = Join-Path $kernelDir "target\x86_64-vahi\debug\bootimage-vahi_kernel.bin"
$kernelElf = Join-Path $kernelDir "kernel\target\x86_64-unknown-none\debug\vahi_kernel"
$ovmf = "C:\Program Files\qemu\OVMF.fd"

if (-not (Test-Path $bootimg)) {
    Write-Error "Bootimage not found. Run make_bootimage.ps1 first."
    exit 1
}

Write-Host "=== SkyOS GDB Debug Session ===" -ForegroundColor Cyan
Write-Host "Bootimage: $bootimg"
Write-Host "Kernel:    $kernelElf"
Write-Host "GDB Port:  $Port"
Write-Host ""

# Launch QEMU with -s (GDB stub on :1234)
$qemuArgs = @(
    "-bios", "`"$ovmf`"",
    "-cpu", "max",
    "-smp", "1",
    "-m", "512M",
    "-no-reboot",
    "-nographic",
    "-drive", "format=raw,file=`"$bootimg`"",
    "-serial", "stdio",
    "-nic", "user",
    "-k", "en-us",
    "-rtc", "base=localtime",
    "-s", "-S"    # GDB server, halt at startup
)

Write-Host "Starting QEMU (halted, waiting for GDB on port $Port)..." -ForegroundColor Yellow
$qemu = Start-Process -NoNewWindow -PassThru -FilePath "qemu-system-x86_64" -ArgumentList $qemuArgs
Write-Host "QEMU PID: $($qemu.Id)"

if (-not $NoGdb) {
    Start-Sleep -Seconds 2
    # Check if GDB is available
    $gdb = Get-Command "rust-lldb" -ErrorAction SilentlyContinue
    if (-not $gdb) { $gdb = Get-Command "lldb" -ErrorAction SilentlyContinue }
    if (-not $gdb) { $gdb = Get-Command "gdb" -ErrorAction SilentlyContinue }

    if ($gdb) {
        Write-Host "Launching $($gdb.Name)..." -ForegroundColor Yellow
        & $gdb.Source -ex "target remote :$Port" -ex "symbol-file `"$kernelElf`"" -ex "break *_start" -ex "continue"
    } else {
        Write-Host "No GDB/LLDB found. QEMU running with -s (port $Port). Connect manually:" -ForegroundColor Yellow
        Write-Host "  gdb -ex 'target remote :$Port' -ex 'symbol-file `"$kernelElf`"'" -ForegroundColor Cyan
    }
} else {
    Write-Host "QEMU running in background. Connect GDB:" -ForegroundColor Cyan
    Write-Host "  gdb -ex 'target remote :$Port' -ex 'symbol-file `"$kernelElf`"'" -ForegroundColor Cyan
    $qemu.WaitForExit()
}
