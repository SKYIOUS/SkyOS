Write-Host "ADE Lint..."
# Prefer clippy; fall back to check.
if (Get-Command "cargo-clippy.exe" -ErrorAction SilentlyContinue) {
    cargo clippy 2>&1
} else {
    Write-Host "clippy not available — falling back to cargo check"
    cargo check 2>&1
}
if ($LASTEXITCODE -ne 0) { Write-Host "Lint failed"; exit 1 }
Write-Host "Lint OK"
