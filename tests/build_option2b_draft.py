#!/usr/bin/env python3
"""Prove the Option 2b userspace draft COMPILES, not just applies.

Pipeline:
  1. Extract the Option 2b diff from ade/docs/kernel-gui-window-fix.md,
     reusing the exact pinned extraction (TestOption2bDocDiff._extract_diff)
     so this script cannot drift from the host test that pins the apply.
  2. Copy the workspace to a scratch dir (excluding .git, target, the
     kernel symlink, fuzz, and local Freebuff state).
  3. git apply the diff there.
  4. cargo build -p login-manager -Zbuild-std=core,alloc
     --target x86_64-sarga.json (the same invocation CI's build job uses).
  5. Exit non-zero with the compiler tail on any failure.

Run from the SkyOS repo root:
    python3 tests/build_option2b_draft.py [scratch_dir]
The scratch dir defaults to a temp dir under $TEMP; pass one to reuse/inspect.
"""
import io
import os
import shutil
import subprocess
import sys
import tempfile

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

# Reuse the pinned extraction (header anchors + 4-space strip + guards).
import test_login_flow as tlf  # noqa: E402

DOC = os.path.join(REPO_ROOT, "ade", "docs", "kernel-gui-window-fix.md")
TARGET_JSON = os.path.join(REPO_ROOT, "x86_64-sarga.json")


def extract_2b_diff():
    extractor = tlf.TestOption2bDocDiff()
    return extractor._extract_diff("## Patch Option 2b")


# Non-workspace-members that would bloat or break a scratch copy:
# kernel is a symlink to the external kernel repo; fuzz is a separate
# harness tree; .freebuff is the local Freebuff DB; the db files are
# local state. None are needed to build a workspace member.
SKIP = {".git", "target", "kernel", "fuzz", ".freebuff",
        "desktop-v2.db", "desktop-v2.db-shm", "desktop-v2.db-wal"}


def make_scratch(dest):
    """Copy the workspace to dest, excluding heavyweight/non-member dirs."""
    if os.path.exists(dest):
        shutil.rmtree(dest)
    os.makedirs(dest)
    for name in os.listdir(REPO_ROOT):
        if name in SKIP:
            continue
        src = os.path.join(REPO_ROOT, name)
        dst = os.path.join(dest, name)
        if os.path.isdir(src) and not os.path.islink(src):
            shutil.copytree(src, dst, symlinks=True)
        else:
            shutil.copy2(src, dst)
    return dest


def main():
    diff = extract_2b_diff()
    print("=== extracted Option 2b diff, bytes=%d ===" % len(diff))
    # Explicit check, not assert: asserts vanish under python -O, and this
    # guard must hold in CI regardless of interpreter flags.
    if "--- a/login-manager/src/main.rs" not in diff:
        print("ERROR: extracted diff does not target login-manager")
        sys.exit(1)

    if len(sys.argv) > 1:
        scratch = os.path.abspath(sys.argv[1])
    else:
        scratch = os.path.join(tempfile.gettempdir(), "option2b-scratch")
    make_scratch(scratch)
    print("=== scratch workspace: %s ===" % scratch)

    # Apply the diff in the scratch.
    r = subprocess.run(
        ["git", "apply", "--whitespace=nowarn", "-"],
        cwd=scratch,
        input=diff.encode("utf-8"),
        capture_output=True,
    )
    if r.returncode != 0:
        print("=== git apply FAILED in scratch ===")
        print(r.stderr.decode("utf-8", "replace")[:2000])
        sys.exit(1)
    print("=== git apply OK ===")

    # Build login-manager exactly like CI's build job does.
    build_cmd = [
        "cargo", "build", "-p", "login-manager",
        "-Zbuild-std=core,alloc", "--target", TARGET_JSON,
    ]
    print("=== cargo build: %s ===" % " ".join(build_cmd))
    r = subprocess.run(build_cmd, cwd=scratch, capture_output=True)
    if r.returncode != 0:
        tail = (r.stderr or r.stdout).decode("utf-8", "replace").strip()[-3000:]
        print("=== CARGO BUILD FAILED ===")
        print(tail)
        print("=== Option 2b draft does NOT compile ===")
        sys.exit(1)
    print("=== CARGO BUILD OK: Option 2b draft compiles ===")
    sys.exit(0)


if __name__ == "__main__":
    main()
