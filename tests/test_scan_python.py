#!/usr/bin/env python3
"""Host-runnable regression gate for scan_python.strip_python itself.

Every Python code-scan pin in tests/ (probe_consume's negative source
pins: "the old whole-buffer scan is gone", "no private Matcher or poll
copy remains") scans Python through scan_python.strip_python. A bug in the
stripping pipeline (a triple-quote left unterminated across lines, an
escaped quote ending a string early, the `#` cut leaking string content)
would silently corrupt ALL of them at once. This file pins the stripping
behavior on synthetic snippets and on one real source file, so a stripping
regression fails HERE with a focused diagnosis instead of a confusing
downstream mismatch.

Run:  python3 tests/test_scan_python.py
"""
import os
import unittest

from scan_python import strip_python

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TESTS_DIR = os.path.join(REPO_ROOT, "tests")
TEST_PROBE_CONSUME_PY = os.path.join(TESTS_DIR, "test_probe_consume.py")
CI = os.path.join(REPO_ROOT, ".github", "workflows", "ci.yml")


class StripPythonBehaviorTest(unittest.TestCase):
    """Synthetic pattern pins for the line-based strip pipeline."""

    def test_double_quoted_strings_stripped(self):
        self.assertNotIn("hello", strip_python('x = "hello"\n'))
        self.assertIn("x = ", strip_python('x = "hello"\n'))

    def test_single_quoted_strings_stripped(self):
        self.assertNotIn("hello", strip_python("x = 'hello'\n"))
        self.assertIn("x = ", strip_python("x = 'hello'\n"))

    def test_triple_double_docstring_stripped(self):
        out = strip_python('"""module doc"""\nimport os\n')
        self.assertNotIn("module doc", out)
        self.assertIn("import os", out)

    def test_triple_single_docstring_stripped(self):
        out = strip_python("'''module doc'''\nimport os\n")
        self.assertNotIn("module doc", out)
        self.assertIn("import os", out)

    def test_multiline_triple_string_stripped(self):
        out = strip_python('x = """line one\nline two"""\ny = 1\n')
        self.assertNotIn("line one", out)
        self.assertNotIn("line two", out)
        self.assertIn("y = 1", out)

    def test_escaped_quote_inside_string(self):
        # An escaped quote must not end the literal early: the whole
        # 'a " b' body goes, and the following code line survives.
        bs = chr(92)
        out = strip_python('x = "a ' + bs + '" b"\ny = 1\n')
        self.assertNotIn("a ", out)
        self.assertIn("y = 1", out)

    def test_hash_comment_stripped_but_code_kept(self):
        out = strip_python("x = 1  # note\nimport os\n")
        self.assertNotIn("note", out)
        self.assertIn("x = 1", out)
        self.assertIn("import os", out)

    def test_shebang_line_dropped(self):
        out = strip_python("#!/usr/bin/env python3\nimport os\n")
        self.assertNotIn("#!/usr/bin/env python3", out)
        self.assertIn("import os", out)

    def test_hash_inside_string_is_still_cut(self):
        # Documented quirk of the line-based strip (see scan_python's
        # docstring): the `#` cut precedes the string scan, so a literal
        # '#' inside a string loses the line tail. Pin it so a future
        # "fix" that changes what downstream pins see is a deliberate,
        # test-visible decision.
        out = strip_python("x = 'a#b'\ny = 1\n")
        self.assertIn("x = ", out)
        self.assertIn("y = 1", out)

    def test_string_terminator_across_lines_stays_open(self):
        # A triple-quote opened on one line must swallow following lines
        # until its closer, not end at the first line break.
        out = strip_python('x = """open\nstill string\n"""\ny = 1\n')
        self.assertNotIn("open", out)
        self.assertNotIn("still string", out)
        self.assertIn("y = 1", out)


class StripPythonRealFileTest(unittest.TestCase):
    """The real consumer: the migrated negative pins must still hold."""

    def test_real_probe_consume_invariants(self):
        with open(TEST_PROBE_CONSUME_PY, encoding="utf-8") as fh:
            code = strip_python(fh.read())
        # Prose mentioning the old whole-buffer primitive must be gone.
        self.assertNotIn("pattern in read_log()", code)
        # The real code tokens the pins rely on survive the strip.
        self.assertIn("class ConsumeSemanticsTest", code)
        self.assertIn("strip_python(", code)  # migrated call sites
        self.assertNotIn("_strip_strings_and_comments", code)


class StripPythonSingleHomeTest(unittest.TestCase):
    """The stripping logic must live in exactly one file: scan_python.py."""

    def test_no_other_test_file_inlines_python_stripping(self):
        offenders = []
        # Self-scan: this file's own "in_triple" probe literal would flag
        # itself, so skip it like scan_python.py.
        this = os.path.basename(__file__)
        for fn in sorted(os.listdir(TESTS_DIR)):
            if not fn.endswith(".py") or fn in ("scan_python.py", this):
                continue
            with open(os.path.join(TESTS_DIR, fn), encoding="utf-8") as fh:
                if "in_triple" in fh.read():
                    offenders.append(fn)
        self.assertEqual(
            offenders, [],
            "no test file may inline Python comment/string stripping;"
            " move it into scan_python.py: %s" % offenders,
        )

    def test_scan_python_owns_the_stripper(self):
        with open(os.path.join(TESTS_DIR, "scan_python.py"), encoding="utf-8") as fh:
            src = fh.read()
        self.assertIn("def strip_python", src)
        self.assertIn("in_triple", src)


class StripPythonCiTest(unittest.TestCase):
    """The gate must run in CI, like its scan_rust sibling."""

    def test_ci_runs_this_gate(self):
        with open(CI, encoding="utf-8") as fh:
            ci = fh.read()
        self.assertIn("python3 tests/test_scan_python.py", ci)


if __name__ == "__main__":
    unittest.main()
