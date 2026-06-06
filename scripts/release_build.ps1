# release_build.ps1 — Production release build with optimizations
$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $PSCommandPath
$repoDir = Resolve-Path "$scriptDir\.."
$kernelDir = Resolve-Path (Join-Path $repoDir "..\SKYIOUS KERNEL")
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$outDir = Join-Path $repoDir "release_$timestamp"

Write-Host "=== SkyOS Release Build ===" -ForegroundColor Cyan
Write-Host "Output: $outDir"
Write-Host ""

# Step 1: Build userspace in release mode
Write-Host "[1/5] Building userspace (release)..." -ForegroundColor Yellow
Push-Location $kernelDir
try {
    $env:RUSTC_BOOTSTRAP = "1"
    cargo build --target "target/x86_64-skyos.json" --release -Z "build-std=core,alloc" -Z "build-std-features=compiler-builtins-mem" --workspace 2>&1 | ForEach-Object { Write-Host "  $_" }
    if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) { throw "userspace build failed" }
} finally { Pop-Location }

# Step 2: Create initrd
Write-Host "[2/5] Creating initrd..." -ForegroundColor Yellow
& ".\build_userspace.ps1" 2>&1 | Out-Null
Write-Host "[OK] initrd ready"

# Step 3: Build kernel (release mode)
Write-Host "[3/5] Building kernel (release)..." -ForegroundColor Yellow
Push-Location (Join-Path $kernelDir "kernel")
try {
    cargo build --target x86_64-unknown-none --release 2>&1 | ForEach-Object { Write-Host "  $_" }
} finally { Pop-Location }

# Step 4: Build bootimage
Write-Host "[4/5] Building bootimage..." -ForegroundColor Yellow
Push-Location $kernelDir
try {
    cargo run --release --manifest-path builder/Cargo.toml 2>&1 | ForEach-Object { Write-Host "  $_" }
} finally { Pop-Location }

# Step 5: Package release
Write-Host "[5/5] Packaging release..." -ForegroundColor Yellow
New-Item -ItemType Directory -Path $outDir -Force | Out-Null
Copy-Item (Join-Path $kernelDir "target\x86_64-vahi\release\bootimage-vahi_kernel.bin") (Join-Path $outDir "bootimage-vahi_kernel.bin") -Force
Copy-Item (Join-Path $kernelDir "target\release\bootimage-vahi_kernel.bin") (Join-Path $outDir) -ErrorAction SilentlyContinue
Compress-Archive -Path (Join-Path $outDir "*") -DestinationPath (Join-Path $outDir "..\skyos-release-$timestamp.zip") -Force
Write-Host ""
Write-Host "=== Release packaged ===" -ForegroundColor Green
Write-Host "Bootimage: $outDir\bootimage-vahi_kernel.bin"
Write-Host "Archive:   $repoDir\skyos-release-$timestamp.zip"
Write-Host ""
Write-Host "Size: $((Get-Item (Join-Path $outDir 'bootimage-vahi_kernel.bin')).Length / 1MB) MB"
