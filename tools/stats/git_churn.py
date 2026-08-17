#!/usr/bin/env python3
"""Per-file commit churn — hotspots where bugs hide.

Usage:
    py tools/stats/git_churn.py [--repo .] [--top 20] [--since 90 days ago]
"""
import argparse
import subprocess
import sys
from collections import Counter


def main(argv=None):
    ap = argparse.ArgumentParser(description="per-file git churn report")
    ap.add_argument("--repo", default=".")
    ap.add_argument("--top", type=int, default=20)
    ap.add_argument("--since", default="90 days ago")
    args = ap.parse_args(argv)

    try:
        out = subprocess.run(
            ["git", "log", f"--since={args.since}", "--name-only", "--pretty=format:"],
            cwd=args.repo, capture_output=True, text=True, check=True).stdout
    except subprocess.CalledProcessError:
        print("git log failed — not a git repo?")
        return 1
    churn = Counter(l for l in out.splitlines() if l.strip())
    print(f"files changed most since '{args.since}':")
    for path, n in churn.most_common(args.top):
        print(f"{n:5d}  {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
