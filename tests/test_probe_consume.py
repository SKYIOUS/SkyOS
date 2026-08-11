#!/usr/bin/env python3
"""Host-runnable pin of expect-style consume semantics in the probe harness.

tests/probe_sendkey.py (the Phase B sendkey probe) is a Python port of the
expect harnesses. Its original wait_for did `if pattern in read_log()` — a
search of the WHOLE accumulated log on every poll, with matched content never
discarded. That produced a false positive: the probe waited for
"[login] window created" TWICE, and the second call passed only because the
first match was still sitting in the buffer — it never proved a second
occurrence.

The fix lives in tests/expect_consume.py (ConsumeMatcher): a match consumes
everything through the end of the match, so a later wait_for can only match
genuinely new data — exactly what expect's buffer discard does. This file
pins that behavior (plus the probe's use of it) so the bug cannot recur.

Run:  python3 tests/test_probe_consume.py
"""
import os
import unittest

from expect_consume import ConsumeMatcher

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = os.path.join(REPO_ROOT, "tests", "probe_sendkey.py")
CI = os.path.join(REPO_ROOT, ".github", "workflows", "ci.yml")


def _read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def _strip_strings_and_comments(src):
    """Remove triple/double-quoted strings and # comments, keeping code tokens.

    Simple lexical strip: drops the shebang, all triple-quoted docstrings,
    string literals, and line comments — enough for a negative source pin
    without a full parser.
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




class ConsumeSemanticsTest(unittest.TestCase):
    """The matcher itself: expect-style discard-on-match."""

    def test_single_occurrence_matches_exactly_once(self):
        # A marker that appears once must satisfy exactly one wait_for.
        m = ConsumeMatcher()
        log = "boot noise\\n[login] window created\\nmore noise\\n"
        self.assertTrue(m.search(log, "[login] window created"))
        self.assertFalse(m.search(log, "[login] window created"),
                         "stale match: the consumed occurrence re-matched")
        self.assertEqual(m.consumed(),
                         log.index("[login] window created") + len("[login] window created"))

    def test_two_occurrences_match_in_order(self):
        m = ConsumeMatcher()
        log = "[login] window created\\n[login] window created\\n"
        self.assertTrue(m.search(log, "[login] window created"))
        self.assertTrue(m.search(log, "[login] window created"),
                        "second genuine occurrence must match")
        self.assertFalse(m.search(log, "[login] window created"))

    def test_search_respects_consume_point_across_growing_buffer(self):
        # The probe re-reads the log file each poll; the buffer grows. A
        # match early in the buffer must not satisfy a later wait even as
        # more data arrives — only data past the consume point counts.
        m = ConsumeMatcher()
        m.search("A[login] window createdB", "[login] window created")
        self.assertFalse(m.search("A[login] window createdB extra", "[login] window created"),
                         "data before the consume point leaked into a later match")
        self.assertTrue(m.search("A[login] window createdB extra[init] x",
                                 "[init] x"))

    def test_consume_advances_past_end_of_match(self):
        m = ConsumeMatcher()
        self.assertTrue(m.search("xxyy", "xx"))
        self.assertEqual(m.consumed(), 2)
        self.assertFalse(m.search("xxyy", "x"))  # all data before 2 is dead

    def test_reset_starts_fresh(self):
        m = ConsumeMatcher()
        m.search("abc", "b")
        m.reset()
        self.assertEqual(m.consumed(), 0)
        self.assertTrue(m.search("abc", "b"))

    def test_shrunken_buffer_does_not_wedge_matcher(self):
        # Truncation safety: if the buffer is ever recreated shorter than
        # the consumed offset, the clamp must recover instead of returning
        # False forever.
        m = ConsumeMatcher()
        log = "prefix [login] window created suffix"
        self.assertTrue(m.search(log, "[login] window created"))
        # Consumed through the match end: 7-char prefix + 22-char marker.
        self.assertEqual(m.consumed(), 7 + len("[login] window created"))
        # Same buffer: nothing new to match.
        self.assertFalse(m.search(log, "[login] window created"))
        # Recreated (shorter) buffer: the stale consume point clamps to the
        # new end instead of wedging find() at -1 forever. A marker in a
        # tiny restarted buffer sits BEFORE the clamped point -> no match.
        self.assertFalse(m.search("tiny", "[login] window created"))
        # The buffer grows again past the clamp: a fresh marker matches.
        grown = "tiny [login] window created"
        self.assertTrue(m.search(grown, "[login] window created"))
        self.assertEqual(m.consumed(), len("tiny ") + len("[login] window created"))


class ProbeContractTest(unittest.TestCase):
    """The probe uses the matcher, not the old whole-buffer scan."""

    @classmethod
    def setUpClass(cls):
        cls.probe = _read(PROBE)

    def test_probe_uses_consume_matcher(self):
        self.assertIn("from expect_consume import ConsumeMatcher", self.probe)
        self.assertIn("MATCHER = ConsumeMatcher()", self.probe)
        self.assertIn("MATCHER.search(", self.probe)

    def test_old_whole_buffer_scan_is_gone(self):
        # The false-positive primitive must not reappear in the probe's CODE.
        # The pin is code-aware: docstrings/comments may legitimately mention
        # the old token in prose (and were reworded when the fix landed), so
        # strip them before asserting — a future doc edit must not false-fail.
        code = _strip_strings_and_comments(self.probe)
        self.assertNotIn("pattern in read_log()", code)

    def test_duplicate_window_wait_removed(self):
        # The regression: the probe waited for "[login] window created"
        # twice; the second call was vacuously green. Exactly one remains.
        self.assertEqual(self.probe.count('wait_for("[login] window created"'), 1)

    def test_ci_host_tests_runs_this_gate(self):
        ci = _read(CI)
        self.assertIn("python3 tests/test_probe_consume.py", ci)


if __name__ == "__main__":
    unittest.main(verbosity=2)
