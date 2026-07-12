$ErrorActionPreference = "Stop"
$KERNEL_DIR = "C:\Users\nanda\Desktop\Github\SKYIOUS KERNEL"
$KERNEL_PATH = "$KERNEL_DIR\target\x86_64-vahi\debug\bootimage-vahi_kernel.bin"
$BIOS_PATH = "$KERNEL_DIR\OVMF.fd"
$LOG_DIR = "$KERNEL_DIR\net_test_logs"
New-Item -ItemType Directory -Force -Path $LOG_DIR | Out-Null

function Start-QemuInstance {
    param($Id, $Mac, $LogFile, $Display)
    $disp = if ($Display) { "-display sdl" } else { "-display none" }
    $tap = "tap$Id"
    Write-Host "Starting QEMU instance $Id (MAC $Mac)..." -ForegroundColor Gray
    $proc = Start-Process -NoNewWindow -PassThru -FilePath "qemu-system-x86_64" -ArgumentList @(
        "-bios", "$BIOS_PATH",
        "-drive", "if=ide,format=raw,file=$KERNEL_PATH",
        "-m", "512M", "-smp", "1", "-vga", "std", "-cpu", "max", "-no-reboot", "-k", "en-us",
        $disp,
        "-serial", "file:$LogFile",
        "-netdev", "tap,id=net0,ifname=$tap,script=no,downscript=no",
        "-device", "e1000,netdev=net0,mac=$Mac"
    )
    $proc
}

function Wait-ForBoot {
    param($LogFile, $TimeoutSeconds = 30)
    $start = Get-Date
    while ((Get-Date) - $start -lt (New-TimeSpan -Seconds $TimeoutSeconds)) {
        Start-Sleep -Milliseconds 500
        if (Test-Path $LogFile) {
            $content = Get-Content -LiteralPath $LogFile -Tail 10
            if ($content -match "SkyOS>" -or $content -match "login:" -or $content -match "# ") {
                return $true
            }
        }
    }
    return $false
}

function Send-Cmd {
    param($Process, $Cmd)
    # No stdin piping available - use serial file log only
    Write-Host "  [would send: $Cmd]" -ForegroundColor DarkGray
}

Write-Host "=== SkyOS Dual-Instance Network Test ===" -ForegroundColor Cyan
Write-Host ""

if ($args[0] -eq "setup") {
    Write-Host "Setting up tap bridges..." -ForegroundColor Yellow
    # Requires admin: create two tap adapters bridged together
    # Manual steps (no automated tap setup on Windows):
    Write-Host "  This script requires two pre-configured tap interfaces." -ForegroundColor Red
    Write-Host "  1. Install OpenVPN TAP driver or use 'tapctl' from tun2socks"
    Write-Host "  2. Create tap0 and tap1 interfaces"
    Write-Host "  3. Bridge them together in Network Control Panel"
    Write-Host ""
    Write-Host "  Alternative: use user-mode networking instead:"
    Write-Host "    qemu-system-x86_64 ... -netdev user,id=net0,hostfwd=tcp::8080-:80"
    Write-Host "    Then from host: curl http://localhost:8080/"
    exit 0
}

if ($args[0] -eq "user") {
    Write-Host "Starting single-instance with port forwarding..." -ForegroundColor Green
    $log = "$LOG_DIR\qemu_user.log"
    Remove-Item $log -ErrorAction SilentlyContinue
    qemu-system-x86_64 `
        -bios "$BIOS_PATH" `
        -drive "if=ide,format=raw,file=$KERNEL_PATH" `
        -m 512M -smp 1 -vga std -cpu max -no-reboot -k en-us `
        -display sdl `
        -serial "file:$log" `
        -netdev user,id=net0,hostfwd=tcp::8080-:80 `
        -device e1000,netdev=net0,mac=52:54:00:12:34:01
    Write-Host "Test from host: curl http://localhost:8080/" -ForegroundColor Cyan
    exit 0
}

if ($args[0] -eq "dual") {
    Write-Host "Starting dual-instance (requires tap0 and tap1)..." -ForegroundColor Green
    $p1 = Start-QemuInstance 1 "52:54:00:12:34:01" "$LOG_DIR\instance1.log" $true
    Start-Sleep -Seconds 2
    $p2 = Start-QemuInstance 2 "52:54:00:12:34:02" "$LOG_DIR\instance2.log" $false
    Write-Host ""
    Write-Host "QEMU instances running." -ForegroundColor Green
    Write-Host "  Instance 1 (display): $LOG_DIR\instance1.log"
    Write-Host "  Instance 2 (headless): $LOG_DIR\instance2.log"
    Write-Host ""
    Write-Host "In instance 1 (the one with display window), run:" -ForegroundColor Cyan
    Write-Host "  httpd 8080"
    Write-Host ""
    Write-Host "In instance 2, once booted, run:" -ForegroundColor Cyan
    Write-Host "  ifconfig"
    Write-Host "  wget http://<ip-of-instance1>:8080/"
    $p1, $p2 | ForEach-Object { $_.WaitForExit() }
    exit 0
}

# Default: show usage
Write-Host "Usage:" -ForegroundColor Yellow
Write-Host "  .\scripts\qemu_net_test.ps1 setup   - Guide for tap bridge setup"
Write-Host "  .\scripts\qemu_net_test.ps1 user    - Single instance with port forwarding (host can curl)"
Write-Host "  .\scripts\qemu_net_test.ps1 dual    - Two instances with tap networking"
Write-Host ""
Write-Host "Quick smoke test path:" -ForegroundColor Green
Write-Host "  1. Build: was: just the full kernel build (which succeeded)"
Write-Host "  2. Launch: .\scripts\qemu_net_test.ps1 user"
Write-Host "  3. In QEMU: ifconfig (should show 10.0.2.15)"
Write-Host "  4. In QEMU: httpd 80"
Write-Host "  5. On host: curl http://localhost:8080/"
Write-Host "     (port will be 8080 on host, forwarded to 80 in guest)"
