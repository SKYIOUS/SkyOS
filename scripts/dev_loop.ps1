# dev_loop.ps1 — Fast iterative development: build userspace → kernel → bootimage → test
param(
    [switch]$Display,      # Use graphical display instead of nographic
    [int]$Timeout = 30,    # Test timeout in seconds
    [string]$KernelDir     # Override kernel dir (default: ../SKYIOUS KERNEL)
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $PSCommandPath
$repoDir = Resolve-Path "$scriptDir\.."
$kernelDir = if ($KernelDir) { $KernelDir } else { Join-Path $repoDir "..\SKYIOUS KERNEL" }
$kernelDir = Resolve-Path $kernelDir

Write-Host "=== SkyOS Dev Loop ===" -ForegroundColor Cyan
Write-Host "Repo:     $repoDir"
Write-Host "Kernel:   $kernelDir"
Write-Host ""

# Step 1: Build userspace
Write-Host "[1/4] Building userspace..." -ForegroundColor Yellow
Push-Location $kernelDir
try {
    & ".\build_userspace.ps1" 2>&1 | ForEach-Object { Write-Host "  $_" }
    if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) { throw "build_userspace.ps1 failed (exit $LASTEXITCODE)" }
} finally { Pop-Location }
Write-Host "[OK] Userspace built" -ForegroundColor Green

# Step 2: Build kernel
Write-Host "[2/4] Building kernel..." -ForegroundColor Yellow
Push-Location (Join-Path $kernelDir "kernel")
try {
    cargo build --target x86_64-unknown-none 2>&1 | ForEach-Object { Write-Host "  $_" }
    if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) { throw "kernel build failed (exit $LASTEXITCODE)" }
} finally { Pop-Location }
Write-Host "[OK] Kernel built" -ForegroundColor Green

# Step 3: Build bootimage
Write-Host "[3/4] Building bootimage..." -ForegroundColor Yellow
Push-Location $kernelDir
try {
    cargo run --manifest-path builder/Cargo.toml 2>&1 | ForEach-Object { Write-Host "  $_" }
} finally { Pop-Location }
Write-Host "[OK] Bootimage ready" -ForegroundColor Green

# Step 4: Run
Write-Host "[4/4] Launching QEMU..." -ForegroundColor Yellow
if ($Display) {
    & ".\run_qemu_display.ps1"
} else {
    & ".\run_test_nographic.ps1"
}
