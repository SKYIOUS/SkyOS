#!/usr/bin/env python3
"""Check trailing whitespace and missing final newlines.

Usage:
    py tools/audit/trailing_ws.py [roots...]

Exits 1 on findings. Ignore with .gitattributes or by removing paths.
"""
import argparse
import os
import sys

SKIP_DIRS = {".git", "target", "node_modules", "__pycache__", ".cache", "release", "build"}
TEXT_EXT = {".rs", ".py", ".c", ".h", ".sh", ".ps1", ".md", ".toml", ".json", ".yml", ".yaml", ".txt"}


def check_file(path: str):
    problems = []
    try:
        with open(path, "rb") as f:
            data = f.read()
    except OSError:
        return problems
    if b"\x00" in data[:2048]:
        return problems
    if data.endswith(b"\n") is False and data:
        problems.append(f"{path}: missing final newline")
    for lineno, line in enumerate(data.splitlines(), 1):
        if line.rstrip(b" \t") != line:
            problems.append(f"{path}:{lineno}: trailing whitespace")
    return problems


def main(argv=None):
    ap = argparse.ArgumentParser(description="check trailing whitespace / missing final newline")
    ap.add_argument("roots", nargs="*", default=["."])
    args = ap.parse_args(argv)

    found = 0
    for root in args.roots:
        for dirpath, dirnames, filenames in os.walk(root):
            dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
            for name in filenames:
                if os.path.splitext(name)[1] in TEXT_EXT:
                    for p in check_file(os.path.join(dirpath, name)):
                        print(p)
                        found += 1
    print(f"{found} problem(s) found")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
