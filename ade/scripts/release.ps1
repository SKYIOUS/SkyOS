param([string]$version="alpha-1")

Write-Host "Building ADE release $version"

cargo check 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) { Write-Host "Build failed"; exit 1 }

Write-Host "ADE $version build OK"
Write-Host "Release artifacts: target/debug/ade.exe"
