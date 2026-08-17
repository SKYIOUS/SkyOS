# SkyOS Login Test — send username+password, expect shell prompt
$scriptDir = Split-Path -Parent $PSCommandPath
$repo = Join-Path $scriptDir "..\kernel"
$qemu = "qemu-system-x86_64"
$bios = Join-Path $scriptDir "..\OVMF.fd"

Write-Host "=== SkyOS Login Test ==="

$script = @"
set timeout 30
spawn $qemu -bios $bios -cpu max -smp 1 -m 512M -no-reboot -nographic `
  -drive format=raw,file="$repo\target\x86_64-vahi\debug\bootimage-vahi_kernel.bin" `
  -serial stdio -nic user -k en-us -rtc base=localtime

expect {
    "login:" { send "root\r"; exp_continue }
    "Password:" { log_user 0; send "skyos\r"; log_user 1; exp_continue }
    "sash\[" { puts "PASS: Got shell prompt"; exit 0 }
    timeout { puts "FAIL: Timeout"; exit 1 }
    eof { puts "FAIL: QEMU exited early"; exit 1 }
}
"@

$tmp = [System.IO.Path]::GetTempFileName()
$script | Out-File -Encoding utf8 "$tmp.exp"
$result = & "expect" "$tmp.exp" 2>&1
Write-Host $result
Remove-Item "$tmp.exp" -Force
