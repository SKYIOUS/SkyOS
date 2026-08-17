#!/usr/bin/env python3
"""Report build artifact sizes (kernel ELF, bootimage, UEFI image, ISO).

Usage:
    py tools/stats/size_report.py [--repo-root .]
"""
import argparse
import os
import sys

KNOWN = [
    "kernel/kernel/target/x86_64-unknown-none/debug/vahi_kernel",
    "kernel/kernel/target/x86_64-unknown-none/release/vahi_kernel",
    "kernel/target/x86_64-vahi/debug/bootimage-vahi_kernel.bin",
    "kernel/target/x86_64-vahi/release/bootimage-vahi_kernel.bin",
    "skyos_uefi.img",
]


def main(argv=None):
    ap = argparse.ArgumentParser(description="report build artifact sizes")
    ap.add_argument("--repo-root", default=".")
    args = ap.parse_args(argv)

    found = 0
    for rel in KNOWN:
        path = os.path.join(args.repo_root, rel)
        if os.path.exists(path):
            size = os.path.getsize(path)
            print(f"{size:>12,}  {rel}")
            found += 1
    if not found:
        print("no artifacts found (build first)")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
