$ErrorActionPreference = "Stop"

# ponytail: was hardcoded to user's home path
$thisDir = $PSScriptRoot
$skyosDir = $thisDir
$kernelDir = if (Test-Path "$thisDir\..\SKYIOUS KERNEL") { Resolve-Path "$thisDir\..\SKYIOUS KERNEL" } else { $null }
if (-not $kernelDir) {
    Write-Host "ERROR: SKYIOUS KERNEL repo not found at sibling path." -ForegroundColor Red
    exit 1
}

# Find the built init binary
$newInit = Get-ChildItem -Recurse -Filter "init" -Path "$skyosDir" | Where-Object { $_.Length -eq 18632 } | Select-Object -First 1
if (!$newInit) {
    Write-Host "ERROR: New init binary not found!" -ForegroundColor Red
    exit 1
}
Write-Host "Found new init: $($newInit.FullName) ($($newInit.Length) bytes)"

# Copy to SkyOS/bin
Copy-Item $newInit.FullName "$kernelDir\SkyOS\bin\init" -Force
Write-Host "Copied to $kernelDir\SkyOS\bin\init"

# Rebuild initrd
python "$kernelDir\build_initrd.py" "$kernelDir\SkyOS"
Write-Host "initrd rebuilt"
