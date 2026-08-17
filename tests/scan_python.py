#!/usr/bin/env python3
"""Comment/string-stripping helper for Python source-contract tests.

Python counterpart of scan_rust.py: exports a single `strip_python(src)`
that removes string literals (single/double/triple-quoted), ``#`` line
comments, and the shebang, so code-scan patterns (negative pins, call-site
topology, guard adjacency) see only the real Python tokens — not prose in
docstrings, comments, or user-facing strings.

Usage::

    from scan_python import strip_python

    src = open("tests/probe_sendkey.py").read()
    code = strip_python(src)
    assert "pattern in read_log()" not in code
"""


def strip_python(src: str) -> str:
    """Remove strings, ``#`` comments, and the shebang from Python source.

    Line-based lexical strip (no full parser): drops any ``#!`` shebang
    line, cuts each line at the first ``#``, then scans the remainder
    char-by-char to remove single-, double-, and triple-quoted string
    literals (triple-quoted strings may span lines). Everything else is
    preserved, including the original line count.

    Documented quirk, preserved deliberately: the ``#`` cut happens BEFORE
    the string scan, so a literal ``#`` inside a string (e.g. ``'a#b'``)
    still loses its line tail. That matches the behavior the negative
    source pins were written against; a full Python tokenizer would be
    needed to do better, and none of the pins require it.
    """
    lines = []
    in_triple = None
    for ln in src.split('\n'):
        if ln.startswith('#!'):
            continue
        code = ln.split('#', 1)[0]
        out = []
        i = 0
        while i < len(code):
            ch = code[i]
            if in_triple:
                if code[i:i + 3] == in_triple:
                    in_triple = None
                    i += 3
                else:
                    i += 1
                continue
            if code[i:i + 3] == '"""' or code[i:i + 3] == "'''":
                in_triple = code[i:i + 3]
                i += 3
                continue
            if ch == '"' or ch == "'":
                q = ch
                i += 1
                while i < len(code) and code[i] != q:
                    if code[i] == '\\':
                        i += 1
                    i += 1
                i += 1
                continue
            out.append(ch)
            i += 1
        lines.append(''.join(out))
    return '\n'.join(lines)
