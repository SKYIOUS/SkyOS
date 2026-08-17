#!/usr/bin/env python3
"""List the largest files in the tree.

Usage:
    py tools/cleanup/bigfiles.py [roots...] [--top 30]
"""
import argparse
import os
import sys

SKIP_DIRS = {".git", "target", "node_modules", "release"}


def main(argv=None):
    ap = argparse.ArgumentParser(description="list largest files")
    ap.add_argument("roots", nargs="*", default=["."])
    ap.add_argument("--top", type=int, default=30)
    args = ap.parse_args(argv)

    files = []
    for root in args.roots:
        for dirpath, dirnames, filenames in os.walk(root):
            dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
            for name in filenames:
                path = os.path.join(dirpath, name)
                try:
                    files.append((os.path.getsize(path), path))
                except OSError:
                    continue
    for size, path in sorted(files, reverse=True)[: args.top]:
        print(f"{size:>12,}  {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
