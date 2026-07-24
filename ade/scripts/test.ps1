Write-Host "ADE Tests (build verification only)..."
cargo check 2>&1
if ($LASTEXITCODE -ne 0) { Write-Host "Verification failed"; exit 1 }
Write-Host "Build verification PASSED"
# Note: no test harness available (#![no_std] + #![no_main])
