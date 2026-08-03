#!/usr/bin/env python3
"""Find near-duplicate source files (copy-paste across the tree).

Compares files by normalized line-shingles (whitespace-stripped, n-line
windows). Reports pairs whose shared shingle ratio exceeds a threshold —
catches files that were copied then lightly edited, which exact-hash
duplicate detection misses.

Usage:
    py tools/duplicate-finder/find_similar_files.py [roots...] [--threshold 0.6]
"""
import argparse
import os
import re
import sys
from collections import defaultdict

EXTENSIONS = {".rs", ".py", ".c", ".h", ".sh", ".ps1", ".md", ".toml", ".json"}
SKIP_DIRS = {".git", "target", "node_modules", "__pycache__", ".cache", "release", ".idea", ".vscode"}


def shingles(text: str, n: int = 5):
    lines = [re.sub(r"\s+", "", ln) for ln in text.splitlines() if ln.strip()]
    return ["|".join(lines[i:i + n]) for i in range(len(lines) - n + 1)]


def scan(root: str):
    files = {}
    for dirpath, dirnames, filenames in os.walk(root):
        # ponytail: don't follow junctions (kernel/ -> separate repo) — double-walks otherwise
        dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS and not os.path.isjunction(os.path.join(dirpath, d))]
        for name in filenames:
            if os.path.splitext(name)[1] not in EXTENSIONS:
                continue
            path = os.path.join(dirpath, name)
            try:
                with open(path, encoding="utf-8", errors="ignore") as f:
                    files[path] = set(shingles(f.read()))
            except OSError:
                continue
    return files


def main(argv=None):
    ap = argparse.ArgumentParser(description="find near-duplicate source files by shingle similarity")
    ap.add_argument("roots", nargs="*", default=["."])
    ap.add_argument("--threshold", type=float, default=0.6)
    ap.add_argument("--min-shingles", type=int, default=10)
    ap.add_argument("--selfcheck", action="store_true")
    args = ap.parse_args(argv)

    if args.selfcheck:
        import tempfile
        lines = ["pub fn common_helper(a: u32, b: u32) -> u32 { a + b }"] * 40
        base = "\n".join(lines)
        with tempfile.TemporaryDirectory() as td:
            with open(os.path.join(td, "one.rs"), "w") as f:
                f.write(base)
            with open(os.path.join(td, "two.rs"), "w") as f:
                f.write(base.replace("a + b", "a.wrapping_add(b)", 1))
            with open(os.path.join(td, "other.rs"), "w") as f:
                f.write("struct CompletelyDifferent;\n" * 40)
            files = {os.path.basename(k): v for k, v in scan(td).items()}
            a, b, c = files["one.rs"], files["two.rs"], files["other.rs"]
            sim_ab = len(a & b) / min(len(a), len(b))
            sim_ac = len(a & c) / min(len(a), len(c))
            assert sim_ab > 0.8, sim_ab
            assert sim_ac < 0.1, sim_ac
        print("selfcheck OK")
        return 0

    files = {}
    for root in args.roots:
        files.update(scan(root))
    paths = list(files)
    found = 0
    for i in range(len(paths)):
        for j in range(i + 1, len(paths)):
            a, b = files[paths[i]], files[paths[j]]
            denom = min(len(a), len(b))
            if denom < args.min_shingles:
                continue
            sim = len(a & b) / denom
            if sim >= args.threshold:
                found += 1
                print(f"{sim:.0%} similar:")
                print(f"  {paths[i]}")
                print(f"  {paths[j]}")
    if found:
        print(f"{found} similar pair(s) found")
        return 1
    print("no near-duplicates")
    return 0


if __name__ == "__main__":
    sys.exit(main())
