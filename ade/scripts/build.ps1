param([string]$config="debug")
Write-Host "Building ADE ($config)..."
cargo check 2>&1
if ($LASTEXITCODE -ne 0) { Write-Host "Build failed"; exit 1 }
Write-Host "Build OK"
