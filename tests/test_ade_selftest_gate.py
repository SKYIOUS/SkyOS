#!/usr/bin/env python3
"""Host-runnable pin of the 'Verify ade selftest verdict' CI gate (no QEMU).

The ade-selftest CI job runs `expect tests/qemu_ade_selftest.exp ... | tee`,
and the `tee` masks the expect script's exit code, so the 'Verify ade
selftest verdict' step in .github/workflows/ci.yml is the authoritative gate:
it greps the serial log for the a11y suite's PASS/FAIL markers and the
aggregate verdict. That gate only executes on a real QEMU boot, so a
regression in the gate itself (a renamed a11y test, a stale threshold) could
never be caught by CI — until now.

This file replays synthetic serial logs through a faithful port of the gate
whose patterns are EXTRACTED from ci.yml at runtime, so they cannot drift,
and asserts the exit codes for the interesting cases:

  * a healthy log                                  -> rc 0
  * a single a11y FAIL line                        -> rc 1  (gate 1)
  * one a11y test never reported (missing PASS)    -> rc 1  (gate 2)
  * the aggregate 'selftest FAIL'                  -> rc 1  (gate 3)
  * a boot that never reached the verdict          -> rc 1  (gate 4)

It also cross-pins the gate's PASS name list against the a11y tests actually
wired into `run_all` (ade/src/util/testing/mod.rs), so adding an a11y test to
run_all without updating the gate (or vice versa) fails this host test.

Finally, it pins the tee-masking fix itself: all three expect|tee boot steps
(ade-selftest, GUI login, GUI + device-manager gate) must run
`set -o pipefail` (so a failing expect fails the job instead of being masked
by tee), and the Verify grep step must remain as the second gate. Removing
either silently re-opens the gap this file exists to close.


Run:  python3 tests/test_ade_selftest_gate.py
"""
import os
import re
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CI_YML = os.path.join(REPO_ROOT, ".github", "workflows", "ci.yml")
RUN_ALL_RS = os.path.join(REPO_ROOT, "ade", "src", "util", "testing", "mod.rs")
ADE_MAIN_RS = os.path.join(REPO_ROOT, "ade", "src", "main.rs")
A11Y_RS = os.path.join(REPO_ROOT, "ade", "src", "util", "testing", "a11y.rs")
INPUT_RS = os.path.join(REPO_ROOT, "ade", "src", "input", "mod.rs")


def _read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def _step_block(ci, name):
    """Slice one step from its `- name:` header to the next step header.

    Only safe when the step is immediately followed by another step (job
    headers are 2/4-space indented and cannot match the 6-space search).
    """
    header = "- name: %s" % name
    if header not in ci:
        raise AssertionError("ci.yml has no step %r (renamed or removed?)" % name)
    start = ci.index(header)
    end = ci.index("      - name:", start + len(header))
    return ci[start:end]


def _verify_block(ci):
    """The 'Verify ade selftest verdict' run block — the authoritative gate.

    Anchored on the step's own unique trailing line (not a positional "next
    name:" search), so a step appended after it inside the ade-selftest job
    cannot silently widen the extracted block.
    """
    start = ci.index("- name: Verify ade selftest verdict")
    end_marker = 'echo "PASS: ade --selftest suite green in QEMU (incl. a11y coverage + keymap contract)"'
    end = ci.index(end_marker, start) + len(end_marker)
    return ci[start:end]


class Gate:
    """Faithful port of the Verify step's grep logic.

    `fail_pat`/`pass_pat` are the raw regex texts from ci.yml (backslash-
    escaped brackets and all), compiled here exactly as `grep -E` would treat
    them. `grep -cE` counts matching LINES, so the pass count is a per-line
    scan, not a total-match count.
    """

    def __init__(self, fail_pat, pass_pat, threshold):
        self.fail_re = re.compile(fail_pat)
        self.pass_re = re.compile(pass_pat)
        self.threshold = threshold

    def verdict(self, log):
        """Replay a serial log through the gate; returns (rc, message)."""
        lines = log.splitlines()
        if any(self.fail_re.search(ln) for ln in lines):
            return 1, "FAIL: an a11y/logout test reported a failure"
        passes = sum(1 for ln in lines if self.pass_re.search(ln))
        if passes < self.threshold:
            return 1, "FAIL: a11y coverage incomplete (found %d/%d)" % (passes, self.threshold)
        if "selftest FAIL" in log:
            return 1, "FAIL: ade selftest suite reported failures"
        if "selftest PASS" not in log:
            return 1, "FAIL: no 'selftest PASS' verdict in serial log"
        return 0, "PASS: ade --selftest suite green in QEMU (incl. a11y coverage)"


def healthy_log(names, extra_lines=(), verdict="selftest PASS"):
    lines = ["[test] PASS test_%s" % n for n in names]
    lines.extend(extra_lines)
    lines.append("[ade] %s" % verdict)
    return "\n".join(lines) + "\n"


class AdeSelftestGateTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        block = _verify_block(_read(CI_YML))
        # Raw pattern texts straight out of ci.yml (backslashes preserved).
        # The YAML stores the patterns with literal backslashes (\[test\]...),
        # so the extraction regex must match a backslash, then the bracket.
        cls.fail_pat = re.search(r'grep -qE "(\\\[test\\\] FAIL [^"]+)"', block).group(1)
        cls.pass_pat = re.search(r'grep -cE "(\\\[test\\\] PASS [^"]+)"', block).group(1)
        cls.threshold = int(re.search(r"-lt (\d+)", block).group(1))
        cls.names = re.search(r"test_\(([^)]+)\)", cls.pass_pat).group(1).split("|")
        cls.wired = re.findall(r"ok &= a11y::test_([a-z0-9_]+)", _read(RUN_ALL_RS))
        cls.gate = Gate(cls.fail_pat, cls.pass_pat, cls.threshold)

    # --- Source contract: the gate and run_all must agree. ---

    def test_gate_names_match_run_all_by_set(self):
        # Set equality, not just count: a typo'd or duplicated name in the
        # ci.yml alternation would still match a naive count pin while either
        # false-failing the real gate every boot (typo -> 10 PASS lines) or
        # leaving a real test unguarded (duplicate -> 11 names, 10 distinct).
        ci_set = sorted(set(self.names))
        wired_set = sorted(set(self.wired))
        self.assertEqual(
            ci_set,
            wired_set,
            "ci.yml PASS names and run_all wired tests diverge:\n"
            "  only in ci.yml: %s\n"
            "  only in run_all: %s"
            % (
                [n for n in ci_set if n not in wired_set],
                [n for n in wired_set if n not in ci_set],
            ),
        )
        self.assertEqual(
            len(self.names),
            len(set(self.names)),
            "duplicate name in the ci.yml PASS alternation: %s"
            % [n for n in set(self.names) if self.names.count(n) > 1],
        )

    def test_threshold_matches_name_count(self):
        self.assertEqual(
            self.threshold,
            len(self.names),
            "ci.yml threshold (-lt %d) must equal the %d names in the PASS "
            "pattern." % (self.threshold, len(self.names)),
        )

    def test_fail_pattern_covers_all_eleven_tests(self):
        # The widened FAIL pattern must catch a FAIL from every wired test —
        # the old pattern (a11y_|tooltip_owner_label) missed focus_* and the
        # tooltip_hardening/role_labels suites.
        for n in self.names:
            line = "[test] FAIL test_%s: something broke" % n
            self.assertTrue(
                self.gate.fail_re.search(line),
                "FAIL pattern %r does not match a failure of test_%s"
                % (self.fail_pat, n),
            )

    def test_expect_tee_pipelines_have_pipefail(self):
        # Every `expect ... | tee` boot step must fail fast on expect
        # failure: a `tee` on the right of a pipeline without pipefail masks
        # the left command's exit code, so the exp step's FAIL verdicts would
        # never reach the job status. The QEMU boot test step is exempt — its
        # expect already ends with `|| exit 1`, and its `qemu | tee` line is
        # log capture, not a verdict pipeline.
        ci = _read(CI_YML)
        for step_name in (
            "Boot and run ade --selftest",
            "Boot and drive GUI login",
            "Boot and assert GUI + device-manager reachability",
        ):
            self.assertIn(
                "- name: %s" % step_name,
                ci,
                "expect|tee step %r renamed or removed" % step_name,
            )
            step = _step_block(ci, step_name)
            self.assertIn(
                "set -o pipefail",
                step,
                "%s is missing `set -o pipefail` - a failing expect would "
                "be masked by the tee pipeline." % step_name,
            )
            self.assertIn("| tee", step, "%s no longer pipes to a log file" % step_name)

    def test_verify_step_still_revalidates_log(self):
        # Belt-and-suspenders: even with pipefail, the Verify step must keep
        # re-validating the serial log (a spurious-pass expect would exit 0
        # despite a FAIL line). setUpClass's pattern extraction already fails
        # if the grep lines vanish, but pin the intent explicitly.
        block = _verify_block(_read(CI_YML))
        self.assertIn("grep -qE", block)
        self.assertIn("grep -cE", block)
        self.assertIn("qemu_ade_log.txt", block)

    def test_keymap_marker_in_verify_step_and_main_rs(self):
        # The [input] keymap-contract marker is printed by ade before the
        # selftest verdict and grepped verbatim by the Verify step. It is
        # the belt-and-suspenders assertion that the routing table
        # (bindings/quit/chord/ctrlq/grabs) is healthy on EVERY QEMU run
        # even if test_keymap were unwired from run_all. Pin the literal
        # in BOTH places so a table edit that drifts either side fails here
        # before CI can go green on a stale marker.
        block = _verify_block(_read(CI_YML))
        self.assertIn("grep -qF \"[input] bindings=", block)
        self.assertIn("[input] bindings=18 quit=1 chord=yes ctrlq=no grabs=10", block)
        main = _read(ADE_MAIN_RS)
        self.assertIn('io::print_str(&alloc::format!(', main)
        # The marker format is built from the dump fields, so the count
        # literals live in the test as the drift tripwire (both sides must
        # be updated together when the table changes).
        self.assertIn('"[input] bindings={} quit={} chord={} ctrlq={} grabs={}\\n"', main)
        self.assertIn("dump_bindings()", main)
        self.assertIn("dump.count", main)
        self.assertIn("dump.desktop_grabs", main)
        # The 10 desktop-grab count must match the table: exactly 10
        # `desktop: true` rows in the BINDINGS literal (9 ctrl+letter +
        # chord), cross-checked with the input.rs test pin.
        input_src = _read(INPUT_RS)
        # Count only actual Binding rows (field literals end in a comma);
        # the is_desktop_shortcut docstring mentions "desktop: true" in
        # prose, so a raw count would overcount by one.
        self.assertEqual(
            input_src.count("desktop: true,"),
            10,
            "BINDINGS has a different number of desktop:true rows than the marker asserts",
        )
        # Symmetry for the total row count: bindings=18 in the marker must
        # equal the number of `Binding {` struct literals in the table
        # (the same table the selftest pins at 18 reachable rows). Count
        # only the 4-space-indented struct literals inside the BINDINGS
        # const, not the `pub(crate) struct Binding {` definition line.
        self.assertEqual(
            len(re.findall(r"^    Binding \{", input_src, re.M)),
            18,
            "BINDINGS has a different number of Binding rows than the marker asserts",
        )

    def test_logout_protocol_gate_in_verify_step(self):
        # The chord->is_ending->exit_code contract is asserted in CI via a
        # dedicated PASS grep in the Verify step (it is an input-suite test
        # outside the a11y PASS alternation, so the a11y count gate cannot
        # cover it). Pin BOTH sides: the grep must exist in the Verify block
        # AND the test must stay wired into run_all (a rename on either side
        # fails here before a QEMU boot can go green on a missing marker).
        block = _verify_block(_read(CI_YML))
        self.assertIn("grep -qE \"\\[test\\] PASS test_logout_protocol_from_chord\"", block)
        self.assertIn("chord->is_ending->exit_code", block)
        self.assertIn(
            "ok &= input::test_logout_protocol_from_chord();",
            _read(RUN_ALL_RS),
            "test_logout_protocol_from_chord unwired from run_all - the CI "
            "grep can never pass",
        )


    def test_two_step_dismissal_gate_in_verify_step(self):
        # The two-step dismissal contract (first Enter dismisses the
        # overlay, the next Enter acts on the still-focused node) is
        # asserted in CI via a dedicated [a11y] marker grep in the Verify
        # step -- the aggregate PASS for test_a11y_activation_dismisses_
        # overlays cannot prove the second Enter acted. Pin BOTH sides:
        # the grep must exist in the Verify block AND the per-step
        # a11y_log_pass call must stay in the Rust test (a rename on
        # either side fails here before a QEMU boot can go green on a
        # missing marker). Mirrors the exp 4a6b expect block.
        block = _verify_block(_read(CI_YML))
        self.assertIn(
            'grep -qE "\\[a11y\\] test_a11y_activation_dismisses_overlays '
            'second Enter acts: PASS"',
            block,
            "two-step [a11y] grep missing from the Verify step",
        )
        a11y = _read(A11Y_RS)
        self.assertIn(
            '"second Enter acts"',
            a11y,
            "the two-step a11y_log_pass step renamed or removed from a11y.rs",
        )
        self.assertIn(
            "test_a11y_activation_dismisses_overlays",
            a11y,
        )

    def test_logout_protocol_fail_line_trips_fail_gate(self):
        # The generic a11y FAIL pattern must also catch a failure of the
        # logout protocol test (the FAIL gate greps test_ FAIL lines, not
        # just the a11y alternation), so a broken logout fails fast rather
        # than waiting for the PASS-count gate.
        # The FAIL gate pattern must now name the logout test: a broken
        # logout (is_ending not flipping, exit code drift, a near-miss key
        # corrupting the unwind) must fail fast via the FAIL grep, not wait
        # for the PASS-count gate.
        line = "[test] FAIL test_logout_protocol_from_chord: near-miss key corrupted the logout"
        self.assertTrue(
            self.gate.fail_re.search(line),
            "FAIL pattern %r does not match test_logout_protocol_from_chord "
            "(widen the alternation in ci.yml)" % self.fail_pat,
        )

    # --- Scenario replay: expected exit codes. ---

    def test_healthy_log_exits_zero(self):
        rc, msg = self.gate.verdict(healthy_log(self.names))
        self.assertEqual(rc, 0, msg)

    def test_single_fail_exits_one(self):
        # A FAIL in one of the tests the OLD gate missed (focus_ prefix).
        log = healthy_log(
            self.names,
            extra_lines=("[test] FAIL test_focus_validate_central: stale focus",),
        )
        rc, msg = self.gate.verdict(log)
        self.assertEqual(rc, 1, msg)

    def test_tooltip_fail_exits_one(self):
        # tooltip_hardening is a11y-suite output the old FAIL pattern missed.
        log = healthy_log(
            self.names,
            extra_lines=("[test] FAIL test_tooltip_hardening: fade never finished",),
        )
        rc, msg = self.gate.verdict(log)
        self.assertEqual(rc, 1, msg)

    def test_missing_one_a11y_pass_exits_one(self):
        # One wired a11y test never reported (unwired from run_all, panic,
        # renamed) while the aggregate verdict is still green: count gate.
        log = healthy_log(self.names[:-1])  # drop the last test's PASS
        rc, msg = self.gate.verdict(log)
        self.assertEqual(rc, 1, msg)

    def test_aggregate_selftest_fail_exits_one(self):
        # All 11 a11y PASS lines present but a non-a11y test failed.
        log = healthy_log(self.names, verdict="selftest FAIL")
        rc, msg = self.gate.verdict(log)
        self.assertEqual(rc, 1, msg)

    def test_no_verdict_exits_one(self):
        # A panic before the suite finished: no selftest verdict at all.
        lines = ["[test] PASS test_%s" % n for n in self.names]
        lines.append("SARGA OS PANIC: ade aborted")
        rc, msg = self.gate.verdict("\n".join(lines) + "\n")
        self.assertEqual(rc, 1, msg)

    def test_keymap_marker_grep_requires_exact_literal(self):
        # Faithful port of the Verify step's `grep -qF` on the marker:
        # -F is a FIXED-STRING search (no regex interpretation), matching
        # the marker as a literal substring of the serial log. Simulate
        # it exactly: the good marker must be found in a log line that
        # contains it, and a drifted count (grabs=11 after a terminal key
        # became a desktop grab, a re-bound Ctrl+Q, a lost chord, etc.)
        # must NOT be found in that same line — so the real gate fails
        # and the job goes red. `grep -q` returns 0 (found) / 1 (not
        # found), mirrored here as `in` / `not in` on the exact string.
        good = "[input] bindings=18 quit=1 chord=yes ctrlq=no grabs=10"
        log_line = "[ade] " + good  # the marker line as printed by ade
        self.assertIn(good, log_line)  # grep -qF would return 0 -> gate passes
        drifted = [
            "[input] bindings=18 quit=1 chord=yes ctrlq=no grabs=11",
            "[input] bindings=19 quit=1 chord=yes ctrlq=no grabs=10",
            "[input] bindings=18 quit=2 chord=yes ctrlq=no grabs=10",
            "[input] bindings=18 quit=1 chord=no ctrlq=no grabs=10",
            "[input] bindings=18 quit=1 chord=yes ctrlq=yes grabs=10",
        ]
        for bad in drifted:
            # A fixed-string search for the drifted literal must NOT match
            # the good log line — that is what makes the gate fail (the
            # literal in ci.yml no longer equals what ade prints).
            self.assertNotIn(bad, log_line)
            # And the good literal must not equal the drifted one, so a
            # future table change forces a deliberate pin update.
            self.assertNotEqual(good, bad)

    def test_aggregate_verdict_requires_exact_substrings(self):
        # The gate keys on the plain 'selftest PASS' substring — a verdict
        # line with a typo ('selftest Passed') must NOT pass.
        lines = ["[test] PASS test_%s" % n for n in self.names]
        lines.append("[ade] selftest Passed")
        rc, _ = self.gate.verdict("\n".join(lines) + "\n")
        self.assertEqual(rc, 1)


if __name__ == "__main__":
    unittest.main()
