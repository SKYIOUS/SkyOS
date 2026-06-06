# update_binaries.ps1 — Sync a specific userspace binary to SkyOS/bin and rebuild initrd
param(
    [Parameter(Mandatory=$true)]
    [string]$Binary     # Binary name (e.g. "init", "sargash", "ls")
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $PSCommandPath
$repoDir = Resolve-Path "$scriptDir\.."
$kernelDir = Resolve-Path (Join-Path $repoDir "..\SKYIOUS KERNEL")
$releaseDir = Join-Path $repoDir "userspace\target\x86_64-skyos\release"
$binDir = Join-Path $kernelDir "SkyOS\bin"
$initrdPy = Join-Path $kernelDir "build_initrd.py"

# If binary is in SkyOS build output, look there too
$altDir = Join-Path $kernelDir "SkyOS\bin"

$src = Join-Path $releaseDir $Binary
if (-not (Test-Path $src)) { $src = Join-Path $altDir $Binary }
if (-not (Test-Path $src)) {
    Write-Error "Binary '$Binary' not found at $releaseDir or $altDir"
    exit 1
}

$dst = Join-Path $binDir $Binary
Write-Host "Syncing $Binary..." -ForegroundColor Yellow
Copy-Item $src $dst -Force

Write-Host "Rebuilding initrd..." -ForegroundColor Yellow
python $initrdPy $kernelDir\SkyOS
if ($LASTEXITCODE -eq 0) {
    Write-Host "[OK] $Binary synced, initrd rebuilt" -ForegroundColor Green
} else {
    Write-Error "initrd rebuild failed"
}
