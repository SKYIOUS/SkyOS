# test_all.ps1 — Run all SkyOS tests
$ErrorActionPreference = "Continue"
$scriptDir = Split-Path -Parent $PSCommandPath
$repoDir = Resolve-Path "$scriptDir\.."
$kernelDir = Resolve-Path (Join-Path $repoDir "..\SKYIOUS KERNEL")

Write-Host "=== SkyOS Test Suite ===" -ForegroundColor Cyan
$passed = 0
$failed = 0
$tests = @()

# Find test scripts
$testScripts = @(
    (Join-Path $kernelDir "tests\test_boot.ps1"),
    (Join-Path $kernelDir "tests\test_panic.ps1")
)
foreach ($ts in $testScripts) {
    if (Test-Path $ts) { $tests += $ts }
}

if ($tests.Count -eq 0) {
    Write-Host "No test scripts found." -ForegroundColor Yellow
    Write-Host "Ensure kernel bootimage is at: $kernelDir\target\x86_64-vahi\debug\bootimage-vahi_kernel.bin" -ForegroundColor Yellow
    exit 0
}

foreach ($t in $tests) {
    $name = Split-Path $t -Leaf
    Write-Host "  Running $name ..." -NoNewline
    $out = & $t 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host " PASS" -ForegroundColor Green
        $passed++
    } else {
        Write-Host " FAIL" -ForegroundColor Red
        $failed++
    }
}

Write-Host ""
Write-Host "=== Results: $passed passed, $failed failed ===" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Red" })
exit $failed
