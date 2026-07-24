Write-Host "=== ADE Check-All ==="
Write-Host ""

Write-Host "--- Build ---"
& "$PSScriptRoot\build.ps1"
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "--- Test ---"
& "$PSScriptRoot\test.ps1"
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "--- Lint ---"
& "$PSScriptRoot\lint.ps1"
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "--- Docs ---"
& "$PSScriptRoot\docs.ps1"

Write-Host ""
Write-Host "=== All checks passed ==="
