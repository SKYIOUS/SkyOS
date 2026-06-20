$repo = "C:\Users\nanda\Desktop\Github\SKYIOUS KERNEL"
$bootimg = "C:\Users\nanda\bootimg.bin"
$bios = "C:\Program Files\qemu\OVMF.fd"
$qemu = "qemu-system-x86_64"

Write-Host "=== SkyOS Boot Test ==="
Write-Host "Boot image: $bootimg"

$outFile = Join-Path $env:TEMP "skyos_nographic.txt"
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $qemu
$psi.Arguments = "-bios `"$bios`" -cpu max -smp 1 -m 512M -no-reboot -nographic -drive format=raw,file=`"$bootimg`" -serial file:`"$outFile`" -nic user -k en-us -rtc base=localtime"
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.CreateNoWindow = $true
$p = [System.Diagnostics.Process]::Start($psi)
$timeout = 30
$elapsed = 0
while (-not $p.HasExited -and $elapsed -lt $timeout) {
    Start-Sleep -Seconds 1
    $elapsed++
}
if (-not $p.HasExited) { $p.Kill(); Write-Host "TIMEOUT after ${timeout}s" }
else { Write-Host "QEMU exited after ${elapsed}s" }
Start-Sleep -Milliseconds 500
if (Test-Path $outFile) {
    $out = Get-Content $outFile -Raw -Encoding utf8
} else { $out = "" }

if ($out -match "login:") { Write-Host "PASS: Boot reached login prompt" -ForegroundColor Green; exit 0 }
else { Write-Host "FAIL: Did not reach login prompt" -ForegroundColor Red; Write-Host ($out -replace "`n","`n  ") }
exit 1
