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

from expect_consume import ConsumeMatcher, EXITED, TIMEOUT
from scan_python import strip_python

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROBE = os.path.join(REPO_ROOT, "tests", "probe_sendkey.py")
CI = os.path.join(REPO_ROOT, ".github", "workflows", "ci.yml")


def _read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


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
        code = strip_python(self.probe)
        self.assertNotIn("pattern in read_log()", code)

    def test_duplicate_window_wait_removed(self):
        # The regression: the probe waited for "[login] window created"
        # twice; the second call was vacuously green. Exactly one remains.
        self.assertEqual(self.probe.count('wait_for("[login] window created"'), 1)

    def test_ci_host_tests_runs_this_gate(self):
        ci = _read(CI)
        self.assertIn("python3 tests/test_probe_consume.py", ci)

class ExpHarnessConsumeTest(unittest.TestCase):
    """Native-expect harnesses: consume-by-design + the guard rails that keep
    the whole-buffer re-match bug out of the exp side.

    The Python port bug was `if pattern in read_log()` re-matching a whole
    accumulated log. The exps avoid that class structurally (expect discards
    the buffer through each match), but three guard rails keep it airtight:

    1. match_max raised past the 2000-byte default everywhere the harness
       greps $expect_out(buffer) at a deadline or after a long boot -
       greps $expect_out(buffer) at a deadline or after a long boot -
       otherwise an early marker rolls out of the window and the fallback
       silently sees only the tail (the giveup gap fixed with the bump).
       The pins are LINE-anchored ('\nmatch_max 1000000\n') because the
       harness comments also mention the literal - a bare assertIn would
       pass after the real command line was deleted.
    2. Deadline buffer greps are guarded by the live-loop flag
       (if {!$gave_up} / if {!$lm_give_up}) so a marker the live arm already
       consumed cannot be re-counted by the fallback.
    3. One-shot sendkey markers ([KBD] IRQ1 fired!) carry an
       `expect -timeout 0` absent-check before the probe key, so a stray
       early-boot copy still in the buffer cannot false-PASS.
    """

    @classmethod
    def setUpClass(cls):
        cls.gui_gate = _read(os.path.join(REPO_ROOT, "tests", "qemu_gui_gate.exp"))
        cls.giveup = _read(os.path.join(REPO_ROOT, "tests", "qemu_giveup_boot.exp"))
        cls.console = _read(os.path.join(REPO_ROOT, "tests", "probe_console_login.exp"))
        cls.iso_probe = _read(os.path.join(REPO_ROOT, "tests", "probe_iso_boot.py"))

    def test_gui_gate_keeps_match_max_and_absent_check(self):
        # gui_gate greps the accumulated buffer for the six /dev node names
        # after login, so the whole boot must stay in the buffer; and its
        # one-shot IRQ1 probe needs the absent-check so a stale buffered
        # copy cannot false-PASS the routing claim.
        self.assertIn("\nmatch_max 1000000\n", self.gui_gate)
        self.assertIn("expect -timeout 0 {", self.gui_gate)

    def test_giveup_keeps_match_max_bump(self):
        # The Aug 2026 fix: the deadline give-up fallbacks grep
        # $expect_out(buffer) (lines ~204/222), which under the 2000-byte
        # default could only see the boot tail - an early 'giving up on svc'
        # marker would be invisible to the fallback. The bump keeps the
        # whole boot in the buffer. Losing it silently narrows the fallback
        # to the tail again.
        self.assertIn("\nmatch_max 1000000\n", self.giveup)

    def test_giveup_buffer_greps_are_flag_guarded(self):
        # The deadline fallbacks must only run when the live loop did NOT
        # already count the marker - otherwise a marker the live arm
        # consumed would be re-matched by the buffer grep (double-count /
        # order-masking). Both guards must stay.
        self.assertIn("if {!$gave_up} {", self.giveup)
        self.assertIn("if {!$lm_give_up} {", self.giveup)

    def test_console_probe_keeps_match_max_and_absent_check(self):
        # probe_console_login greps the accumulated buffer for '[vahid]
        # ready' after 'login:' (marker prints early in boot) and probes the
        # one-shot IRQ1 marker, so it needs both guard rails too.
        self.assertIn("\nmatch_max 1000000\n", self.console)
        self.assertIn("expect -timeout 0 {", self.console)

    def test_no_exec_whole_file_reread_in_exps(self):
        # The exp-side equivalent of `pattern in read_log()` would be
        # `exec grep` on a tee'd logfile - a whole-file re-scan that never
        # discards. None of the harnesses may do it (all matching goes
        # through the expect buffer, which consumes).
        for name, src in (("gui_gate", self.gui_gate),
                          ("giveup", self.giveup),
                          ("console", self.console)):
            for ln in src.splitlines():
                stripped = ln.strip()
                if stripped.startswith("#"):
                    continue
                self.assertNotIn("exec grep", stripped,
                                 "%s exp re-reads a logfile (whole-buffer scan)" % name)

    def test_iso_probe_is_single_pass(self):
        # probe_iso_boot.py boots once, reads the serial file once, and
        # checks each marker exactly once against that single snapshot - no
        # repeated wait over a growing buffer, so the re-match bug class
        # cannot occur. Pin the single-pass shape: no wait_for, no sleep
        # polling, no loop re-reading the log.
        self.assertNotIn("wait_for(", self.iso_probe)
        self.assertNotIn("time.sleep", self.iso_probe)
        self.assertNotIn("while ", self.iso_probe)


class PollDriverTest(unittest.TestCase):
    """ConsumeMatcher.poll_with_timeout: the shared serial-driver poll loop."""

    def test_hit_returns_the_check_result_as_is(self):
        m = ConsumeMatcher()
        reads = []
        result = m.poll_with_timeout(
            lambda text: "HIT" if "x" in text else None,
            timeout=5, read=lambda: reads.append(1) or "ax", sleep=0)
        self.assertEqual(result, "HIT")
        self.assertEqual(len(reads), 1)

    def test_exited_outcome(self):
        m = ConsumeMatcher()
        result = m.poll_with_timeout(
            lambda text: None, timeout=5, read=lambda: "x",
            poll=lambda: True, sleep=0)
        self.assertEqual(result, EXITED)

    def test_timeout_outcome(self):
        m = ConsumeMatcher()
        result = m.poll_with_timeout(
            lambda text: None, timeout=0.01, read=lambda: "x",
            poll=lambda: False, sleep=0)
        self.assertEqual(result, TIMEOUT)

    def test_poll_hook_not_required(self):
        # boot_stress-style use: no read hook, the check closure owns its
        # log reads. The text arg is None; the check still fires.
        m = ConsumeMatcher()
        calls = []
        result = m.poll_with_timeout(
            lambda text: (calls.append(text) or True), timeout=5, sleep=0)
        self.assertIs(result, True)
        self.assertEqual(calls, [None])

    def test_no_reads_after_exit_outcome(self):
        # Once the poll hook reports the process ended, the loop stops
        # polling instead of grinding to the deadline.
        m = ConsumeMatcher()
        reads = []
        result = m.poll_with_timeout(
            lambda text: None, timeout=5,
            read=lambda: reads.append(1) or "x",
            poll=lambda: len(reads) > 1, sleep=0)
        self.assertEqual(result, EXITED)
        self.assertLessEqual(len(reads), 2)


class SharedDriverContractTest(unittest.TestCase):
    """All three QEMU-facing poll loops call the shared driver, not inline
    copies of the deadline/exit/sleep skeleton."""

    @classmethod
    def setUpClass(cls):
        cls.expect_consume = _read(os.path.join(REPO_ROOT, "tests",
                                                 "expect_consume.py"))
        cls.probe = _read(PROBE)
        cls.local = _read(os.path.join(REPO_ROOT, "tests",
                                       "run_ade_selftest_local.py"))
        cls.stress = _read(os.path.join(REPO_ROOT, "tests", "boot_stress.py"))

    def test_driver_method_and_outcomes_exist(self):
        self.assertIn(
            "def poll_with_timeout(self, check, *, timeout, read=None, poll=None,\n"
            "                          sleep=0.5):",
            self.expect_consume)
        self.assertIn('EXITED = "EXITED"', self.expect_consume)
        self.assertIn('TIMEOUT = "TIMEOUT"', self.expect_consume)

    def test_probe_sendkey_uses_shared_driver(self):
        self.assertIn("MATCHER.poll_with_timeout(", self.probe)
        self.assertNotIn("while time.time() < deadline",
                         strip_python(self.probe))

    def test_run_ade_selftest_uses_shared_driver(self):
        self.assertIn("from expect_consume import ConsumeMatcher", self.local)
        self.assertIn("self.m.poll_with_timeout(", self.local)
        code = strip_python(self.local)
        self.assertNotIn("class Matcher", code)  # private duplicate gone
        self.assertNotIn("while time.time() < deadline", code)

    def test_boot_stress_uses_shared_driver(self):
        self.assertIn("ConsumeMatcher().poll_with_timeout(", self.stress)
        self.assertNotIn("while time.monotonic() < deadline",
                         strip_python(self.stress))

    def test_no_private_matcher_or_poll_copy_remains(self):
        # The shared module is the single home of consume semantics and the
        # poll skeleton: no QEMU-facing script may carry its own copy.
        for name, src in (("probe", self.probe), ("local", self.local),
                          ("stress", self.stress)):
            self.assertNotIn("class Matcher",
                             strip_python(src), name)


if __name__ == "__main__":
    unittest.main(verbosity=2)
