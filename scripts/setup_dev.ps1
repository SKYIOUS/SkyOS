# setup_dev.ps1 — One-click SkyOS development environment setup (Windows)
param([switch]$Force)

$ErrorActionPreference = "Continue"
Write-Host "=== SkyOS Dev Environment Setup ===" -ForegroundColor Cyan

# 1. Check Rust
$rust = Get-Command rustc -ErrorAction SilentlyContinue
if (-not $rust) {
    Write-Host "[INSTALL] Rust not found. Install from https://rustup.rs first." -ForegroundColor Red
    exit 1
}
$ver = rustc --version
Write-Host "[OK] Rust: $ver"

# 2. Install nightly + components
Write-Host "[SETUP] Installing nightly toolchain..."
rustup toolchain install nightly --allow-downgrade -c rust-src -c llvm-tools-preview 2>&1 | Out-Null
Write-Host "[OK] nightly toolchain ready"

# 3. Install targets
$targets = @("x86_64-unknown-none", "x86_64-skyos")
foreach ($t in $targets) {
    rustup target add $t --toolchain nightly 2>&1 | Out-Null
    Write-Host "[OK] target $t"
}

# 4. Check QEMU
$qemu = Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
if (-not $qemu) {
    Write-Host "[WARN] QEMU not found in PATH. Install from https://www.qemu.org/download/" -ForegroundColor Yellow
} else {
    Write-Host "[OK] QEMU: $($qemu.Source)"
}

# 5. Check Python
$py = Get-Command python -ErrorAction SilentlyContinue
if ($py) { Write-Host "[OK] Python: $((python --version) 2>&1)" }
else { Write-Host "[WARN] Python not found" -ForegroundColor Yellow }

# 6. Install bootimage tool
if ($Force -or -not (Get-Command cargo-bootimage -ErrorAction SilentlyContinue)) {
    Write-Host "[SETUP] Installing cargo-bootimage..."
    cargo install bootimage 2>&1 | Out-Null
}
Write-Host "[OK] bootimage ready"

# 7. Check OVMF
$ovmf = "C:\Program Files\qemu\OVMF.fd"
if (Test-Path $ovmf) { Write-Host "[OK] OVMF BIOS found" }
else { Write-Host "[WARN] OVMF.fd not at $ovmf" -ForegroundColor Yellow }

Write-Host ""
Write-Host "=== Dev environment ready ===" -ForegroundColor Green
Write-Host "  Build:          .\build.ps1"
Write-Host "  Boot image:     cd ..\SKYIOUS KERNEL; .\make_bootimage.ps1"
Write-Host "  Run (display):  .\run_qemu_display.ps1"
Write-Host "  Run (nographic): .\run_test_nographic.ps1"
