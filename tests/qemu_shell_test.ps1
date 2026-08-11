# SkyOS QEMU Shell Test
# Boots QEMU, logs in via serial, runs commands, checks output.
param(
    [string]$KernelDir = "",
    [string]$IsoPath = "",
    [int]$TimeoutSeconds = 180
)

$scriptDir = Split-Path -Parent $PSCommandPath
if (-not $KernelDir) {
    $KernelDir = Join-Path $scriptDir "..\kernel"
}

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$SkyOSDir = Split-Path -Parent $ScriptDir
$PassCount = 0
$FailCount = 0

Write-Host "=== SkyOS QEMU Shell Test ===" -ForegroundColor Cyan

# If no ISO path given, build everything
if (-not $IsoPath) {
    Write-Host "--- Building userspace ---"
    Push-Location $SkyOSDir
    cargo build "-Zbuild-std=core,alloc" --target x86_64-sarga.json --release
    if (-not $?) { Write-Host "FAIL: userspace build failed" -ForegroundColor Red; exit 1 }
    Pop-Location

    Write-Host "--- Building initrd ---"
    Push-Location $SkyOSDir
    python3 build_initrd.py
    if (-not $?) { Write-Host "FAIL: initrd build failed" -ForegroundColor Red; exit 1 }
    Pop-Location

    $initrdDest = Join-Path $KernelDir "SkyOS"
    New-Item -ItemType Directory -Force -Path $initrdDest | Out-Null
    Copy-Item (Join-Path $SkyOSDir "initrd.tar") (Join-Path $initrdDest "initrd.tar") -Force
}

# Start QEMU
$qemuArgs = @(
    "-bios", "OVMF.fd",
    "-m", "512M",
    "-smp", "2",
    "-nographic", "-no-reboot",
    "-serial", "mon:stdio",
    "-device", "e1000,netdev=net0",
    "-netdev", "user,id=net0"
)
if ($IsoPath) {
    $qemuArgs += @("-cdrom", "`"$IsoPath`"")
} else {
    $bootImg = Join-Path $KernelDir "target\x86_64-vahi\debug\bootimage-vahi_kernel.bin"
    $qemuArgs += @("-drive", "format=raw,file=`"$bootImg`"")
}

Write-Host "Starting QEMU..." -ForegroundColor Yellow

$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = "qemu-system-x86_64"
$psi.Arguments = ($qemuArgs -join " ")
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardInput = $true
$psi.CreateNoWindow = $true

$p = [System.Diagnostics.Process]::Start($psi)
if (-not $p) { Write-Host "FAIL: Could not start QEMU" -ForegroundColor Red; exit 1 }

# NOTE on password echo: this harness writes via StandardInput and captures
# only StandardOutput. The guest never echoes (kernel serial is TX-only;
# login reads without writing back), so the password never lands in the
# captured stream — unlike the expect harnesses, which echo their own send
# and wrap the password in log_user 0. No suppression needed here.
$output = New-Object System.Text.StringBuilder
$commands = @(
    @{ after = "login:";        send = "root`r" }
    @{ after = "Password:";     send = "skyos`r" }
    @{ after = "sash\\[";       send = "ls /`r" }
    @{ after = "bin";           send = "uname -a`r" }
    @{ after = "Vahi|sarga-os"; send = "echo SHELL_TEST_OK`r" }
    @{ after = "SHELL_TEST_OK"; send = "ls /bin/ | head -10`r" }
    @{ after = "sash|init|cat"; send = "/bin/futex_test`r" }
    @{ after = "PASS";          send = "exit`r" }
)

$cmdIdx = 0
$elapsed = 0
$done = $false

while (-not $p.HasExited -and $elapsed -lt $TimeoutSeconds) {
    if ($p.StandardOutput.Peek() -ge 0) {
        $line = $p.StandardOutput.ReadLine()
        if ($line) {
            $output.AppendLine($line) | Out-Null
            Write-Host "  $line" -ForegroundColor Gray

            if ($cmdIdx -lt $commands.Count) {
                $cmd = $commands[$cmdIdx]
                if ($line -match $cmd.after) {
                    Start-Sleep -Milliseconds 500
                    $p.StandardInput.WriteLine($cmd.send)
                    Write-Host ">>> $($cmd.send)" -ForegroundColor Green
                    $cmdIdx++
                    if ($cmdIdx -eq $commands.Count) {
                        $done = $true
                        break
                    }
                }
            }
        }
    } else {
        Start-Sleep -Milliseconds 100
        $elapsed++
    }
}

if (-not $p.HasExited) { $p.Kill() }
$fullOutput = $output.ToString()

# Check results
Write-Host "`n=== Results ===" -ForegroundColor Cyan

$tests = @(
    @{ name = "Boot to login";         pattern = "login:" }
    @{ name = "Shell prompt";          pattern = "sash\\[" }
    @{ name = "ls / output";           pattern = "bin" }
    @{ name = "uname output";          pattern = "Vahi|sarga-os" }
    @{ name = "echo test";             pattern = "SHELL_TEST_OK" }
    @{ name = "futex_test binary";     pattern = "PASS" }
)

$allPass = $true
foreach ($t in $tests) {
    if ($fullOutput -match $t.pattern) {
        Write-Host "  PASS: $($t.name)" -ForegroundColor Green
        $PassCount++
    } else {
        Write-Host "  FAIL: $($t.name) (expected '$($t.pattern)')" -ForegroundColor Red
        $FailCount++
        $allPass = $false
    }
}

Write-Host "`n$PassCount passed, $FailCount failed" -ForegroundColor Cyan
if ($allPass) {
    Write-Host "QEMU SHELL TEST: PASS" -ForegroundColor Green
    exit 0
} else {
    Write-Host "QEMU SHELL TEST: FAIL" -ForegroundColor Red
    exit 1
}
