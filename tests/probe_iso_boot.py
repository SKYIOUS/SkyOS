#!/usr/bin/env python3
"""Boot an ISO in QEMU with OVMF and assert it reaches the getty login prompt.

Mirrors the release-iso.yml boot_capture gate exactly (-bios OVMF.fd,
-cdrom, 512M/2 cores, -display none -monitor none -serial file:), so the
local verdict is the same one CI would produce: PASS if 'login:' appears in
the serial capture within the timeout.

Run:  python3 tests/probe_iso_boot.py <path-to.iso> [seconds=90]
"""

import os
import re
import subprocess
import sys
import tempfile


def probe(iso, seconds):
    fd, log_path = tempfile.mkstemp(prefix="iso_boot_", suffix=".log")
    os.close(fd)
    cmd = [
        "qemu-system-x86_64",
        "-bios", "OVMF.fd",
        "-cdrom", iso,
        "-m", "512M", "-smp", "2",
        "-display", "none", "-monitor", "none",
        "-serial", f"file:{log_path}",
        "-device", "e1000,netdev=net0", "-netdev", "user,id=net0",
        "-no-reboot",
    ]
    try:
        subprocess.run(cmd, timeout=seconds)
    except subprocess.TimeoutExpired:
        pass  # intentional: capture whatever serial the boot produced
    with open(log_path, "rb") as f:
        raw = f.read()
    os.unlink(log_path)
    # The serial stream may be raw ANSI; decode lossily for greps.
    text = raw.decode("utf-8", errors="replace")
    markers = {
        "login:": "login:" in text,
        "[init] starting service:": "[init] starting service:" in text,
        "[vahid] ready": "[vahid] ready" in text,
        "[ade] session established": "[ade] session established" in text,
        "[login] failed to create window": "[login] failed to create window" in text,
    }
    return text, markers


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    iso = sys.argv[1]
    seconds = int(sys.argv[2]) if len(sys.argv) > 2 else 90
    if not os.path.exists(iso):
        print(f"ERROR: {iso} not found")
        return 1
    print(f"=== booting {os.path.basename(iso)} ({os.path.getsize(iso):,} B) for {seconds}s ===")
    text, markers = probe(iso, seconds)
    for k, v in markers.items():
        print(f"  {'PASS' if v else '--- '} {k}")
    if markers["login:"]:
        print(f"PASS: {os.path.basename(iso)} reached the getty login prompt")
        # Show the login-adjacent context (init/vahid/ade lines).
        for ln in text.splitlines():
            if re.search(r"\[(init|vahid|ade|login)\]|login:", ln):
                print("   |", ln.strip()[:110])
        return 0
    print(f"FAIL: {os.path.basename(iso)} did NOT reach login: in {seconds}s")
    print("--- serial tail ---")
    print(text[-800:])
    return 1


if __name__ == "__main__":
    sys.exit(main())
