#!/usr/bin/env python3
"""SkyOS boot stress test.

Boots the kernel in QEMU repeatedly and fails on any stall, panic, or
failed kernel self-test (TAP `not ok`). Catches boot-time races and lock
regressions (e.g. the IrqSafeMutex drop-order bug that intermittently hung
the CPU spinning on the ALLOCATOR).

Usage:
    py tests/boot_stress.py [--tries 40] [--image <bootimage.bin>]
                            [--timeout 35] [--keep-logs <dir>]

Exit code 0 = all boots passed; 1 = any stall/panic/TAP failure.
"""

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_IMAGE = (
    REPO_ROOT / "kernel" / "target" / "x86_64-vahi" / "debug"
    / "bootimage-vahi_kernel.bin"
)

PASS_TOKEN = "starting service"
FAIL_TOKENS = ("not ok", "Bail out!", "KERNEL PANIC", "Panicked")


def build_qemu_cmd(image: Path, logfile: Path, root: Path, ovmf: Path,
                   smp: int = 1, cpu: str = "max") -> list[str]:
    return [
        "qemu-system-x86_64",
        "-bios", str(ovmf),
        "-cpu", cpu,
        "-smp", str(smp),
        "-m", "512M",
        "-nographic",
        "-drive", f"format=raw,file={image}",
        "-serial", f"file:{logfile}",
        "-nic", "user",
        "-k", "en-us",
        "-rtc", "base=localtime",
        "-no-reboot",
    ]


def check_log(logfile: Path) -> tuple[bool, str]:
    """Returns (passed, last_line_or_failure_reason)."""
    try:
        text = logfile.read_text(errors="replace")
    except FileNotFoundError:
        return False, "<no log produced>"
    for token in FAIL_TOKENS:
        if token.lower() in text.lower():
            return False, f"log contains '{token}'"
    if PASS_TOKEN in text:
        return True, ""
    lines = [l for l in text.splitlines() if l.strip()]
    return False, lines[-1] if lines else "<empty log>"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--tries", type=int, default=40)
    ap.add_argument("--image", type=Path, default=DEFAULT_IMAGE)
    ap.add_argument("--timeout", type=float, default=35.0)
    ap.add_argument("--smp", type=int, default=1,
                    help="QEMU -smp CPU count (2+ needs --cpu qemu64,-smep on "
                         "QEMU builds whose TCG stalls on AP CR4.SMEP writes)")
    ap.add_argument("--cpu", type=str, default="max", help="QEMU -cpu model")
    ap.add_argument("--keep-logs", type=Path, default=None,
                    help="directory to save per-try serial logs into")
    args = ap.parse_args()

    if not args.image.exists():
        print(f"ERROR: boot image not found: {args.image}")
        print("Build it first: py build_disk.py --kernel-only")
        return 1

    ovmf = REPO_ROOT / "OVMF.fd"
    if not ovmf.exists():
        print(f"ERROR: OVMF.fd not found at {ovmf}")
        return 1

    keep_dir = None
    if args.keep_logs:
        keep_dir = args.keep_logs
        keep_dir.mkdir(parents=True, exist_ok=True)

    tmp = Path(tempfile.mkdtemp(prefix="skyos_boot_stress_"))
    failed = 0
    for try_no in range(1, args.tries + 1):
        logfile = tmp / f"boot_{try_no}.log"
        proc = subprocess.Popen(
            build_qemu_cmd(args.image, logfile, REPO_ROOT, ovmf,
                           args.smp, args.cpu),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        deadline = time.monotonic() + args.timeout
        ok, reason = False, ""
        while time.monotonic() < deadline and proc.poll() is None:
            ok, reason = check_log(logfile)
            if ok:
                break
            time.sleep(0.5)
        if proc.poll() is None:
            proc.kill()
            proc.wait()
        if not ok:
            ok, reason = check_log(logfile)
        if ok:
            print(f"try {try_no}/{args.tries}: PASS")
        else:
            failed += 1
            print(f"try {try_no}/{args.tries}: FAIL ({reason})")
            if keep_dir:
                shutil.copy2(logfile, keep_dir / f"fail_{try_no}.log")
            # Keep going to measure frequency, unless the caller wants a
            # quick single-shot failure (--tries 1).
            if args.tries == 1:
                break
    shutil.rmtree(tmp, ignore_errors=True)

    if failed:
        print(f"RESULT: {failed}/{args.tries} boots FAILED")
        return 1
    print(f"RESULT: {args.tries}/{args.tries} boots passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
