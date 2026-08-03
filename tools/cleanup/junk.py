#!/usr/bin/env python3
"""Find junk files (cache, temp, build leftovers) — preview before deleting.

Usage:
    py tools/cleanup/junk.py [roots...] [--delete] [--min-size 0]
"""
import argparse
import os
import sys

SKIP_DIRS = {".git"}
JUNK_NAMES = {"__pycache__", ".pyc", ".tmp", ".bak", "Thumbs.db", ".DS_Store", "*.pyc"}
JUNK_DIRS = {"__pycache__", ".cache"}
JUNK_EXTS = {".pyc", ".tmp", ".bak", ".log~", ".orig", ".rej"}


def scan(root: str):
    junk = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS and d not in JUNK_DIRS]
        for name in filenames:
            if name in JUNK_NAMES or os.path.splitext(name)[1] in JUNK_EXTS:
                path = os.path.join(dirpath, name)
                try:
                    junk.append((os.path.getsize(path), path))
                except OSError:
                    continue
    return junk


def main(argv=None):
    ap = argparse.ArgumentParser(description="find junk/cache/temp files")
    ap.add_argument("roots", nargs="*", default=["."])
    ap.add_argument("--delete", action="store_true", help="delete found files")
    ap.add_argument("--min-size", type=int, default=0)
    args = ap.parse_args(argv)

    junk = []
    for root in args.roots:
        junk += scan(root)
    junk = [(s, p) for s, p in junk if s >= args.min_size]
    total = sum(s for s, _ in junk)
    for size, path in sorted(junk, reverse=True):
        print(f"{size:>10,}  {path}")
    print(f"{len(junk)} junk file(s), {total:,} bytes")
    if args.delete:
        for _, path in junk:
            try:
                os.remove(path)
            except OSError as e:
                print(f"failed: {path}: {e}")
        print("deleted")
    return 0


if __name__ == "__main__":
    sys.exit(main())
