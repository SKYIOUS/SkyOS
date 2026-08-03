#!/usr/bin/env python3
"""Find exact duplicate files by content hash.

Usage:
    py tools/duplicate-finder/find_duplicates.py [roots...] [--min-size 1024]

Exit code 1 when duplicates are found (CI-friendly). Non-trivial logic has
a self-check: run with --selfcheck.
"""
import argparse
import hashlib
import os
import sys
from collections import defaultdict

DEFAULT_MIN_SIZE = 1024
SKIP_DIRS = {".git", "target", "node_modules", "__pycache__", ".cache", "release"}


def scan(root: str, min_size: int):
    by_hash = defaultdict(list)
    for dirpath, dirnames, filenames in os.walk(root):
        # ponytail: don't follow junctions (kernel/ -> separate repo) — double-walks otherwise
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS and not os.path.isjunction(os.path.join(dirpath, d))]
        for name in filenames:
            path = os.path.join(dirpath, name)
            try:
                if os.path.getsize(path) < min_size:
                    continue
                h = hashlib.sha256()
                with open(path, "rb") as f:
                    for chunk in iter(lambda: f.read(1 << 20), b""):
                        h.update(chunk)
                by_hash[h.hexdigest()].append(path)
            except OSError:
                continue
    return {k: v for k, v in by_hash.items() if len(v) > 1}


def main(argv=None):
    ap = argparse.ArgumentParser(description="find exact duplicate files by content hash")
    ap.add_argument("roots", nargs="*", default=["."])
    ap.add_argument("--min-size", type=int, default=DEFAULT_MIN_SIZE)
    ap.add_argument("--selfcheck", action="store_true")
    args = ap.parse_args(argv)

    if args.selfcheck:
        import tempfile
        with tempfile.TemporaryDirectory() as td:
            for name, content in (("a.txt", "x" * 100), ("b.txt", "x" * 100), ("c.txt", "y" * 100)):
                with open(os.path.join(td, name), "w") as f:
                    f.write(content)
            groups = scan(td, 0)
            assert len(groups) == 1, groups
            assert len(list(groups.values())[0]) == 2, groups
            with open(os.path.join(td, "b.txt"), "w") as f:
                f.write("z" * 100)
            assert scan(td, 0) == {}, "no dupes after change"
        print("selfcheck OK")
        return 0

    found = 0
    for root in args.roots:
        for h, paths in sorted(scan(root, args.min_size).items(), key=lambda kv: -len(kv[1])):
            found += 1
            print(f"{len(paths)} copies of {h[:12]}:")
            for p in paths:
                print(f"  {p}")
    if found:
        print(f"{found} duplicate group(s) found")
        return 1
    print("no duplicates")
    return 0


if __name__ == "__main__":
    sys.exit(main())
