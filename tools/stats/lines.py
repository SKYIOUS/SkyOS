#!/usr/bin/env python3
"""Line counts by extension and top files.

Usage:
    py tools/stats/lines.py [roots...] [--top 20]
"""
import argparse
import os
import sys
from collections import Counter

SKIP_DIRS = {".git", "target", "node_modules", "__pycache__", ".cache", "release", "build"}


def scan(root: str):
    by_ext = Counter()
    files = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in filenames:
            path = os.path.join(dirpath, name)
            try:
                with open(path, encoding="utf-8", errors="ignore") as f:
                    n = sum(1 for _ in f)
            except OSError:
                continue
            ext = os.path.splitext(name)[1] or "(none)"
            by_ext[ext] += n
            files.append((n, path))
    return by_ext, files


def main(argv=None):
    ap = argparse.ArgumentParser(description="line counts by extension and top files")
    ap.add_argument("roots", nargs="*", default=["."])
    ap.add_argument("--top", type=int, default=20)
    args = ap.parse_args(argv)

    by_ext = Counter()
    files = []
    for root in args.roots:
        e, f = scan(root)
        by_ext += e
        files += f
    total = sum(by_ext.values())
    print(f"{total:>10,} total lines")
    for ext, n in by_ext.most_common():
        print(f"{n:>10,}  {ext or '(none)'}")
    print("\ntop files:")
    for n, path in sorted(files, reverse=True)[: args.top]:
        print(f"{n:>10,}  {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
