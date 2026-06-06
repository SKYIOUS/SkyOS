# clean_all.ps1 — Remove all build artifacts from both repos
$ErrorActionPreference = "Continue"
$scriptDir = Split-Path -Parent $PSCommandPath
$repoDir = Resolve-Path "$scriptDir\.."
$kernelDir = Resolve-Path (Join-Path $repoDir "..\SKYIOUS KERNEL")

Write-Host "=== Clean All Build Artifacts ===" -ForegroundColor Yellow

# SkyOS repo artifacts
$paths = @(
    (Join-Path $repoDir "target"),
    (Join-Path $repoDir "aethos.img"),
    (Join-Path $repoDir "initrd.tar"),
    (Join-Path $repoDir "sarga.vdi"),
    (Join-Path $repoDir "staging\bin\init"),
    (Join-Path $repoDir "staging\bin\sash"),
    (Join-Path $repoDir "*.log"),
    (Join-Path $repoDir "*.txt")
)
foreach ($p in $paths) {
    if (Test-Path $p) { Remove-Item $p -Recurse -Force; Write-Host "  Removed: $p" }
}

# SKYIOUS KERNEL artifacts
$kpaths = @(
    (Join-Path $kernelDir "target\x86_64-unknown-none"),
    (Join-Path $kernelDir "target\x86_64-vahi"),
    (Join-Path $kernelDir "target\debug"),
    (Join-Path $kernelDir "target\release"),
    (Join-Path $kernelDir "SkyOS\initrd.tar"),
    (Join-Path $kernelDir "SkyOS\bin"),
    (Join-Path $kernelDir "bootimage-vahi_kernel.bin"),
    (Join-Path $kernelDir "bootimage-velox_kernel.bin"),
    (Join-Path $kernelDir "vahi_uefi.img"),
    (Join-Path $kernelDir "vahi.vdi"),
    (Join-Path $kernelDir "*.log")
)
foreach ($p in $kpaths) {
    if (Test-Path $p) { Remove-Item $p -Recurse -Force; Write-Host "  Removed: $p" }
}

Write-Host "[OK] All build artifacts cleaned" -ForegroundColor Green
Write-Host "  Rebuild with: .\make_bootimage.ps1 (SKYIOUS KERNEL)" -ForegroundColor Cyan
