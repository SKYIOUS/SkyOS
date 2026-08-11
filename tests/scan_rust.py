#!/usr/bin/env python3
"""Comment/string-stripping helper for Rust source-contract tests.

Exports a single `strip_rust(src)` that masks string literals, then strips
line and block comments, so code-scan patterns (return counting, call-site
topology, guard adjacency) see only the real Rust tokens — not prose in
comments or user-facing strings.

Usage::

    from scan_rust import strip_rust

    src = open("login/src/main.rs").read()
    code = strip_rust(src)
    calls = code.count("note_failed_attempt(")
"""

import re


def strip_rust(src: str) -> str:
    """Strip comments and mask string literals from Rust source.

    Order matters: string literals are masked FIRST so that ``//`` and
    ``/*`` inside strings (e.g. ``\"// not a comment\"``) survive the
    comment pass. Line comments are stripped second, then block comments.
    Returns a string where strings become empty quoted pairs (``\"\"`` /
    ``''``) and comments become empty; everything else is unchanged.
    """
    code = re.sub(r'"(?:\\.|[^"\\])*"', '""', src)     # mask string literals
    code = re.sub(r"//[^\n]*", "", code)                  # strip line comments
    code = re.sub(r"/\*.*?\*/", "", code, flags=re.S)   # strip block comments
    return code