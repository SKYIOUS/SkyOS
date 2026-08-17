#!/usr/bin/env python3
"""Audit TODO/FIXME/HACK/XXX markers across the tree.

Usage:
    py tools/audit/markers.py [roots...] [--kind todo] [--top 30]

Exits 1 when markers are found (CI-friendly).
"""
import argparse
import os
import re
import sys
from collections import Counter

SKIP_DIRS = {".git", "target", "node_modules", "__pycache__", ".cache", "release", "build"}
MARKER_RE = re.compile(r"(TODO|FIXME|HACK|XXX|BUG)\b")


def scan(root: str):
    hits = []
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
        for name in filenames:
            path = os.path.join(dirpath, name)
            try:
                with open(path, encoding="utf-8", errors="ignore") as f:
                    for lineno, line in enumerate(f, 1):
                        for m in MARKER_RE.finditer(line):
                            hits.append((m.group(1), path, lineno, line.strip()[:100]))
            except OSError:
                continue
    return hits


def main(argv=None):
    ap = argparse.ArgumentParser(description="audit TODO/FIXME/HACK/XXX markers")
    ap.add_argument("roots", nargs="*", default=["."])
    ap.add_argument("--kind", default=None, help="only this marker kind (TODO, FIXME, ...)")
    ap.add_argument("--top", type=int, default=0, help="print only top-N by file")
    args = ap.parse_args(argv)

    hits = []
    for root in args.roots:
        hits += scan(root)
    if args.kind:
        hits = [h for h in hits if h[0].lower() == args.kind.lower()]

    if args.top:
        by_file = Counter(h[1] for h in hits)
        for path, count in by_file.most_common(args.top):
            print(f"{count:4d}  {path}")
    else:
        for kind, path, lineno, text in hits:
            print(f"{kind:6s} {path}:{lineno}  {text}")

    print(f"{len(hits)} marker(s) found")
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main())
