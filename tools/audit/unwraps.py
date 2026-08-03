#!/usr/bin/env python3
"""Count unwrap()/expect() in kernel sources (AGENTS.md bans them).

Usage:
    py tools/audit/unwraps.py [roots...] [--top 30]
"""
import argparse
import os
import re
import sys
from collections import Counter

SKIP_DIRS = {".git", "target", "node_modules", "__pycache__", ".cache", "release", "build"}
UNWRAP_RE = re.compile(r"\.(unwrap|expect)\(\)")


def scan(root: str):
    counts = Counter()
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in filenames:
            if not name.endswith(".rs"):
                continue
            path = os.path.join(dirpath, name)
            try:
                with open(path, encoding="utf-8", errors="ignore") as f:
                    counts[path] = len(UNWRAP_RE.findall(f.read()))
            except OSError:
                continue
    return counts


def main(argv=None):
    ap = argparse.ArgumentParser(description="count unwrap()/expect() in Rust sources")
    ap.add_argument("roots", nargs="*", default=["."])
    ap.add_argument("--top", type=int, default=30)
    args = ap.parse_args(argv)

    counts = Counter()
    for root in args.roots:
        counts += scan(root)
    total = sum(counts.values())
    for path, n in counts.most_common(args.top):
        print(f"{n:4d}  {path}")
    print(f"{total} total unwrap/expect call sites")
    return 1 if total else 0


if __name__ == "__main__":
    sys.exit(main())
