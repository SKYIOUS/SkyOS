#!/usr/bin/env python3
"""Host-runnable pins for the GUI gate's boot-time memory-pressure marker.

The QEMU harness tests/qemu_gui_gate.exp asserts GUI reachability
("[login] window created" vs the "[login] failed to create window" respawn
loop). To settle whether that failure is persistent or transient OOM
(kernel-gui-window-fix.md Option 1 vs Option 2), login-manager now prints
"[login] mem free=N pages" from ctlFS /ctl/sys/mem/free (the kernel buddy
allocator's live free-page count) right before Window::create. The gate
captures the marker and reports it in its verdicts, and CI asserts it.

These pins hold the source/exp/CI contract so the marker cannot silently
drift while the kernel rewrite is in flight:

* login-manager reads /ctl/sys/mem/free and prints the marker BEFORE
  Window::create (the number is the free count at allocation time).
* the gate exp captures the marker and reports free pages in the FAIL arm.
* the ci.yml Verify step greps the marker from the captured log.

Run:  python3 tests/test_gui_gate_mem_marker.py
"""
import os
import unittest

from scan_rust import strip_rust

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LM_RS = os.path.join(REPO_ROOT, "login-manager", "src", "main.rs")
EXP = os.path.join(REPO_ROOT, "tests", "qemu_gui_gate.exp")
CI = os.path.join(REPO_ROOT, ".github", "workflows", "ci.yml")


def _read(p):
    with open(p, encoding="utf-8") as fh:
        return fh.read()


class GuiGateMemMarkerContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.lm = _read(LM_RS)
        cls.lm_code = strip_rust(cls.lm)
        cls.exp = _read(EXP)
        cls.ci = _read(CI)

    def test_login_manager_reads_ctlfs_free_before_create(self):
        # The marker must read the live buddy-allocator free count from
        # ctlFS — the same node the kernel's own terminal reads — so the
        # number is real kernel memory state, not a userspace guess.
        self.assertIn("/ctl/sys/mem/free", self.lm)
        # The print must carry the exact grep-able prefix CI and the exp
        # match on: "[login] mem free=N pages".
        self.assertIn("[login] mem free=", self.lm)
        # Ordering pin: the marker's read_to_string appears in user_main
        # BEFORE the Window::create call, so the captured free count is the
        # state at allocation time (what the Option 1 vs 2 question needs).
        # strip_rust masks string literals, so both calls are
        # read_to_string("") in the stripped code; the ctlfs read is the
        # LAST one in the file (verify_password's shadow read comes first).
        marker_pos = self.lm_code.rfind('read_to_string("")')
        create_pos = self.lm_code.find('Window::create(""')
        self.assertNotEqual(marker_pos, -1, "ctlfs read missing")
        self.assertNotEqual(create_pos, -1, "Window::create missing")
        self.assertLess(
            marker_pos,
            create_pos,
            "memory marker must print before Window::create",
        )

    def test_gate_exp_captures_and_reports_marker(self):
        # The verdict loop must passively capture the free count...
        self.assertIn("-re {\\[login\\] mem free=(\\d+) pages}", self.exp)
        self.assertIn("set mem_free $expect_out(1,string)", self.exp)
        # ...and the FAIL arm for the respawn loop must report it, so a
        # failing boot's log carries the memory-pressure evidence.
        self.assertIn("buddy free=$mem_free pages at create", self.exp)
        # The final verdict must carry the marker too (or warn when absent).
        self.assertIn("buddy free=$mem_free pages at login-manager create", self.exp)
        self.assertIn("memory-pressure marker never appeared", self.exp)

    def test_ci_verify_step_asserts_marker_presence(self):
        # The Verify step must grep the marker from the captured log as an
        # explicit positive assertion — a future exp edit that drops the
        # marker fails the job even if the aggregate verdict string stays.
        self.assertIn('grep -q "\\[login\\] mem free=" qemu_gui_gate_log.txt', self.ci)
        self.assertIn("FAIL: memory-pressure marker missing from the gate log", self.ci)

    def test_no_allowed_dead_code_introduced(self):
        # The whole workspace is gated on #[allow(dead_code)] absence; the
        # marker helper must not smuggle one in (it is called, so none is
        # expected — this guards against a future refactor leaving one).
        self.assertNotIn("#[allow(dead_code)]", self.lm)

    def test_host_tests_step_wired(self):
        # This file itself must be run by the host-tests job — it was
        # created before its CI step and nearly went unwired. A future
        # edit that drops the step fails the suite, not just CI.
        self.assertIn(
            "python3 tests/test_gui_gate_mem_marker.py", self.ci,
            "host-tests job lost the GUI gate memory marker step",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
