#!/usr/bin/env python3
"""Host-runnable pins for the init respawn-accounting serial-log gate.

The QEMU harness tests/qemu_giveup_boot.exp asserts init's boundedness
claims against REAL serial markers: svc's non-zero exit (Usage path, every
boot) must drive init to "[init] giving up on svc after too many crashes",
while login-manager's clean exit(0) window-failure loop must NEVER produce
"giving up on login-manager". The kernel is mid-major-change and init's
waitpid currently reaps no exits (live boot Aug 10, 2026: services died -
svc exit 1, login-manager SEGV 0x0 - yet no "[init] service X exited"), so
the harness probes exit delivery first and reports KERNEL-GATED until the
kernel lands a working reap. These pins hold the harness/CI/source contract
so nothing drifts while the kernel rewrite is in flight.

Run:  python3 tests/test_giveup_gate.py
"""
import os
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
EXP = os.path.join(REPO_ROOT, "tests", "qemu_giveup_boot.exp")
CI = os.path.join(REPO_ROOT, ".github", "workflows", "ci.yml")
INIT_RS = os.path.join(REPO_ROOT, "init", "src", "main.rs")
SVC_RS = os.path.join(REPO_ROOT, "svc", "src", "main.rs")
LM_RS = os.path.join(REPO_ROOT, "login-manager", "src", "main.rs")


def _read(p):
    with open(p, encoding="utf-8") as fh:
        return fh.read()


class GiveUpGateContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.exp = _read(EXP)
        cls.ci = _read(CI)
        cls.init = _read(INIT_RS)
        cls.svc = _read(SVC_RS)
        cls.lm = _read(LM_RS)

    def test_exp_has_live_probe_and_verdict_lines(self):
        # The harness must (a) probe exit delivery, (b) enforce the two
        # claims, (c) report the kernel gate honestly when reaping is dead.
        self.assertIn(r"(?s)\[init\] service .*?exited", self.exp)
        self.assertIn("PASS: bounded - init gave up on svc", self.exp)
        # The vahid arm: the original request's 'giving up on vahid'
        # assertion, fires when a forced-failure boot reaches vahid's fatal
        # /dev-creation path.
        self.assertIn(r"(?s)\[init\] giving up on .*?vahid", self.exp)
        self.assertIn("PASS: unbounded - no give-up on login-manager", self.exp)
        self.assertIn("KERNEL-GATED:", self.exp)

    def test_svc_is_the_real_nonzero_exit_service(self):
        # init spawns svc without argv, so argc < 2 always -> Usage + return
        # 1: svc is the one service that exits non-zero on EVERY boot. This
        # is why the boundedness marker is "giving up on svc", not vahid
        # (vahid's fatal path needs a /dev node creation failure that no
        # stock boot hits).
        self.assertIn("Usage: svc", self.svc)
        self.assertIn("return 1;", self.svc[self.svc.index("Usage: svc"):])

    def test_login_manager_failure_path_is_clean_exit_zero(self):
        # The window-failure arm returns 0 (clean exit -> crashes reset ->
        # give-up can never fire): the unboundedness the exp asserts. A
        # future edit that returns non-zero here would burn MAX_RESPAWNS.
        idx = self.lm.index("failed to create window")
        self.assertIn("return 0;", self.lm[idx:])

    def test_init_marker_sources_match_exp_patterns(self):
        # The markers the exp keys on must match init/src/main.rs's
        # write_all calls byte for byte (3-part writes + TTY0W interleave).
        self.assertIn('b"[init] service "', self.init)
        self.assertIn('b"[init] giving up on "', self.init)
        self.assertIn('b" after too many crashes\\n"', self.init)

    def test_ci_boots_the_new_exp(self):
        # The gate job must actually run the harness and capture its log.
        self.assertIn("qemu_giveup_boot.exp", self.ci)
        self.assertIn("qemu_giveup_log.txt", self.ci)

    def test_ci_verify_step_is_conditional_on_kernel_gate(self):
        # The Verify step must defer when the kernel gate holds (no exit
        # delivery) and enforce both verdict lines once it is lifted.
        block = self.ci[self.ci.index("Verify bounded/unbounded"):]
        self.assertIn("KERNEL-GATED:", block)
        self.assertIn('grep -q "PASS: bounded"', block)
        self.assertIn('grep -q "PASS: unbounded"', block)
        # The deferral must be surfaced in the CI UI, not a silent exit 0.
        self.assertIn("::warning::", block)
        # Belt-and-suspenders: the raw log must show a giving-up marker and
        # no login-manager give-up (grep -z spans the TTY0W interleave).
        self.assertIn('grep -q "\\[init\\] giving up on "', block)
        self.assertIn("grep -zqE", block)

    def test_host_tests_job_runs_this_pin(self):
        # This host test must be wired into CI's host-tests job.
        self.assertIn("test_giveup_gate.py", self.ci)


if __name__ == "__main__":
    unittest.main()
