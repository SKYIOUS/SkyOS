# one-shot gate: kernel build -> bootimage -> boot stress (the verified sequence)
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)   # tools/gate -> repo root
$kernel = Join-Path $repo "kernel"
if (-not (Test-Path (Join-Path $kernel "kernel\Cargo.toml"))) {
    Write-Error "kernel checkout not found at $kernel (junction expected)"
}
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Write-Host "== building kernel (self_test) =="
Push-Location (Join-Path $kernel "kernel")
try { cargo +nightly build --features net,smp,ai_rule,self_test } finally { Pop-Location }
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== regenerating bootimage =="
Push-Location $repo
try { py -c "import build_disk; build_disk.build_bootimage(r'$repo', r'$kernel')" } finally { Pop-Location }
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "== boot stress (10 tries) =="
Push-Location $repo
try { py tests\boot_stress.py --tries 10 } finally { Pop-Location }
exit $LASTEXITCODE
