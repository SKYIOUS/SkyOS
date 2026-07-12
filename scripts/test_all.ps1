# test_all.ps1 — Run all SkyOS tests
$ErrorActionPreference = "Continue"
$scriptDir = Split-Path -Parent $PSCommandPath
$repoDir = Resolve-Path "$scriptDir\.."
$kernelDir = Resolve-Path (Join-Path $repoDir "..\SKYIOUS KERNEL")

Write-Host "=== SkyOS Test Suite ===" -ForegroundColor Cyan
$passed = 0
$failed = 0
$tests = @()

# Phase 1: cargo build and unit tests
Write-Host "`n[Phase 1] Build + Unit tests" -ForegroundColor Yellow
Push-Location $repoDir
cargo build -Zbuild-std=core,alloc --target x86_64-sarga.json 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "  Build: PASS" -ForegroundColor Green; $passed++
} else {
    Write-Host "  Build: FAIL" -ForegroundColor Red; $failed++
}
cargo test -p libsarga 2>&1 | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "  Unit tests: PASS" -ForegroundColor Green; $passed++
} else {
    Write-Host "  Unit tests: FAIL" -ForegroundColor Red; $failed++
}
Pop-Location

# Phase 2: QEMU boot tests
Write-Host "`n[Phase 2] QEMU boot tests" -ForegroundColor Yellow
$testScripts = @(
    (Join-Path $kernelDir "tests\test_boot.ps1"),
    (Join-Path $kernelDir "tests\test_panic.ps1")
)
foreach ($ts in $testScripts) {
    if (Test-Path $ts) { $tests += $ts }
}
if ($tests.Count -eq 0) {
    Write-Host "  (no kernel test scripts found - build kernel first)" -ForegroundColor Yellow
}
foreach ($t in $tests) {
    $name = Split-Path $t -Leaf
    Write-Host "  Running $name ..." -NoNewline
    $out = & $t 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host " PASS" -ForegroundColor Green; $passed++
    } else {
        Write-Host " FAIL" -ForegroundColor Red; $failed++
    }
}

Write-Host "`n=== Results: $passed passed, $failed failed ===" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Red" })
exit $failed
