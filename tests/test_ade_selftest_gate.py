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
INPUT_TESTS_RS = os.path.join(REPO_ROOT, "ade", "src", "util", "testing", "input.rs")
SESSION_RS = os.path.join(REPO_ROOT, "ade", "src", "service", "session.rs")
SESSION_TESTS_RS = os.path.join(REPO_ROOT, "ade", "src", "util", "testing", "session.rs")
ADE_MAIN_RS = os.path.join(REPO_ROOT, "ade", "src", "main.rs")
A11Y_RS = os.path.join(REPO_ROOT, "ade", "src", "util", "testing", "a11y.rs")
DESKTOP_RS = os.path.join(REPO_ROOT, "ade", "src", "core", "desktop.rs")
A11Y_MOD_RS = os.path.join(REPO_ROOT, "ade", "src", "sec", "a11y", "mod.rs")
NODE_RS = os.path.join(REPO_ROOT, "ade", "src", "sec", "a11y", "node.rs")
WINDOW_RS = os.path.join(REPO_ROOT, "ade", "src", "core", "window.rs")
TASKBAR_RS = os.path.join(REPO_ROOT, "ade", "src", "core", "taskbar.rs")
NOTIF_OVERLAY_RS = os.path.join(REPO_ROOT, "ade", "src", "render", "notification_overlay.rs")
RENDER_MOD_RS = os.path.join(REPO_ROOT, "ade", "src", "render", "mod.rs")
INPUT_RS = os.path.join(REPO_ROOT, "ade", "src", "input", "mod.rs")
SELFTEST_EXP = os.path.join(REPO_ROOT, "tests", "qemu_ade_selftest.exp")
SELFTEST_LOCAL = os.path.join(REPO_ROOT, "tests", "run_ade_selftest_local.py")


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


def _extract(block, pat, what):
    """Extract one needle from the Verify block, failing with a named
    message when it is missing -- a future editor that drops a dedicated
    grep must get a clear pin failure, not an obscure AttributeError."""
    m = re.search(pat, block)
    if m is None:
        raise AssertionError(
            "Verify block lost the %s (extraction pattern %r matched nothing)"
            % (what, pat)
        )
    return m.group(1)


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

    def add_dedicated(self, logout_pat, window_pat, input_names):
        self.logout_re = re.compile(logout_pat)
        self.window_re = re.compile(window_pat)
        self.input_names = input_names

    def verdict(self, log):
        """Replay a serial log through the gate; returns (rc, message).
        Ports the fail-re, a11y count, dedicated logout/window/input
        greps, and the aggregate verdict -- the keymap-marker and getty-
        cap greps are deliberately out of scope (each has its own test)."""
        lines = log.splitlines()
        if any(self.fail_re.search(ln) for ln in lines):
            return 1, "FAIL: an a11y/logout test reported a failure"
        passes = sum(1 for ln in lines if self.pass_re.search(ln))
        if passes < self.threshold:
            return 1, "FAIL: a11y coverage incomplete (found %d/%d)" % (passes, self.threshold)
        # Dedicated greps: a missing PASS marker in ANY input-suite test
        # fails the job even when the a11y count and verdict are green.
        if not any(self.logout_re.search(ln) for ln in lines):
            return 1, "FAIL: logout protocol test did not pass (chord->is_ending->exit_code)"
        if not any(self.window_re.search(ln) for ln in lines):
            return 1, "FAIL: window-open logout inertness test did not pass (Quit no-op branch)"
        for name in self.input_names:
            if not any(("PASS %s" % name) in ln for ln in lines):
                return 1, "FAIL: input-suite test %s did not pass" % name
        if "selftest FAIL" in log:
            return 1, "FAIL: ade selftest suite reported failures"
        if "selftest PASS" not in log:
            return 1, "FAIL: no 'selftest PASS' verdict in serial log"
        return 0, "PASS: ade --selftest suite green in QEMU (incl. a11y coverage)"


def healthy_log(names, extra_lines=(), verdict="selftest PASS"):
    lines = ["[test] PASS test_%s" % n for n in names]
    # Dedicated PASS markers the Verify step greps beyond the a11y count
    # (logout protocol, window-open companion, input-suite for-loop).
    lines.append("[test] PASS test_logout_protocol_from_chord")
    lines.append("[test] PASS test_logout_inert_with_window_open")
    lines.append("[test] PASS test_keymap")
    lines.append("[test] PASS test_from_raw")
    lines.append("[test] PASS test_session_end_gate")
    lines.append("[test] PASS test_session_end_protocol")
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
        # Dedicated PASS greps beyond the a11y alternation (logout protocol,
        # window-open companion, and the input-suite for-loop) -- extracted
        # from the same Verify block so they stay in lockstep with ci.yml.
        cls.logout_pat = _extract(
            block,
            r'grep -qE "\\\[test\\] PASS (test_logout_protocol_from_chord)"',
            "logout-protocol dedicated grep",
        )
        cls.window_pat = _extract(
            block,
            r'grep -qE "\\\[test\\] PASS (test_logout_inert_with_window_open)"',
            "window-open dedicated grep",
        )
        cls.input_names = _extract(
            block,
            r"for t in (test_[a-z0-9_ ]+); do",
            "input-suite for-loop",
        ).split()
        cls.gate = Gate(cls.fail_pat, cls.pass_pat, cls.threshold)
        cls.gate.add_dedicated(cls.logout_pat, cls.window_pat, cls.input_names)

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

    def test_hardware_esc_probe_pinned(self):
        # The real-hardware Esc session-end probe in qemu_ade_selftest.exp:
        # after the selftest verdict the harness launches ade interactively
        # and injects a REAL Esc via the QEMU monitor (sendkey esc), which
        # must unwind the session with the rich marker - proving the
        # byte-path session-end contract (a11y Esc arm, 0x1B) on real input,
        # not just the synthetic pins in test_session_end_gate. Pin the
        # probe's machinery and markers so a future edit that drops the
        # probe, the monitor helper, or the asserted marker fails here
        # before any QEMU run. Also mirror in the local runner so local
        # verification cannot drift from CI.
        exp = _read(SELFTEST_EXP)
        self.assertIn('proc monitor_cmd {cmd}', exp,
                      "qemu_ade_selftest.exp lost the QEMU monitor helper")
        self.assertIn('send \"\\x01c\"', exp,
                      "qemu_ade_selftest.exp lost the monitor toggle (Ctrl-A c)")
        self.assertIn('monitor_cmd \"sendkey esc\"', exp,
                      "qemu_ade_selftest.exp lost the hardware Esc injection")
        self.assertIn('session established', exp)
        self.assertIn('session ended code=0 ending=true', exp,
                      "qemu_ade_selftest.exp lost the rich session-end marker")
        # The dedicated logout-protocol PASS grep (4a14): the
        # chord -> is_ending -> exit_code contract asserted in CI
        # alongside the a11y greps - and its Esc twin covers the byte
        # path too. A future edit that drops the block must fail here.
        self.assertIn('{\\[test\\] PASS test_logout_protocol_from_chord}', exp,
                      "qemu_ade_selftest.exp lost the logout-protocol PASS grep")
        self.assertIn('logout protocol (chord + Esc byte path) coverage passed', exp,
                      "qemu_ade_selftest.exp lost the logout coverage PASS message")
        self.assertIn('ade could not create its GUI window', exp,
                      "qemu_ade_selftest.exp lost the GUI-gate FAIL arm")
        local = _read(SELFTEST_LOCAL)
        self.assertIn('def monitor_cmd(self, cmd)', local,
                      "local runner lost the monitor helper")
        self.assertIn('sendkey esc', local,
                      "local runner lost the hardware Esc injection")
        self.assertIn('session ended code=0 ending=true', local,
                      "local runner lost the session-end marker")

    def test_logout_esc_window_guard_pinned(self):
        # The Esc twin in test_logout_protocol_from_chord now ends with the
        # window-open guard: with a window present, Esc is dismiss-only and
        # must NOT start the session end - mirroring the chord's guard in
        # test_logout_inert_with_window_open, so both session-end keys
        # share the same empty-desktop precondition (the a11y Esc arm's
        # `wm.is_empty()` no-op branch, driven through the real byte path).
        # Pin the leg's FAIL strings + window name so a future edit that
        # drops or weakens the guard fails host-side before any boot.
        src = _read(INPUT_TESTS_RS)
        self.assertIn("esc with window open started the session end", src,
                      "Esc-twin window-open guard lost its session-end assertion")
        self.assertIn("esc with window open closed the window", src,
                      "Esc-twin window-open guard lost its window-survival check")
        self.assertIn("esc with window open did not dismiss the overlay", src,
                      "Esc-twin dismiss-only leg lost its overlay check")
        self.assertIn("esc dismissed the window with the overlay", src,
                      "Esc-twin dismiss-only leg lost its window-survival check")
        self.assertIn('AppWindow::new(100, 100, 400, 300, "EscGuard")', src,
                      "Esc-twin window-open guard lost its guard window")


    def test_session_end_protocol_idempotency_pinned(self):
        # SessionManager's end protocol is structurally idempotent: the
        # a11y Esc arm can re-enter request_end any number of times (second
        # Esc press, near-miss sweep keys) without mutating the unwind.
        # request_end is a single monotonic store to a private bool and
        # exit_code returns a compile-time constant that reads no state, so
        # every re-entry observes the same is_ending()/exit_code() as the
        # first. Pin both the documented mechanism (session.rs) and the
        # test legs that prove it behaviorally (testing/session.rs).
        svc = _read(SESSION_RS)
        self.assertIn("Idempotency is structural, not guarded", svc,
                      "session.rs lost the request_end structural-idempotency doc")
        self.assertIn("reads no state and takes no input", svc,
                      "session.rs lost the exit_code state-free doc")
        tst = _read(SESSION_TESTS_RS)
        self.assertIn("exit code nonzero before ending", tst,
                      "boot-state exit_code pin lost (state-free contract)")
        self.assertIn("re-entry mutated the ending state or exit code", tst,
                      "re-entry storm leg lost its mutation FAIL")
        self.assertIn("prior_ok", tst,
                      "re-entry storm leg lost its stability tracker")


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
        # Full grep line, not just the marker literal: the marker also
        # appears in the step's comment, so a bare-literal needle would
        # pass even if the actual `grep -qF` drifted back to 18 (the
        # comment staying at 17 masks it). Anchor on the whole grep
        # command so the enforced literal is the one that runs.
        self.assertIn(
            'if ! grep -qF "[input] bindings=17 quit=1 chord=yes ctrlq=no grabs=10" qemu_ade_log.txt; then',
            block,
        )
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
        # Symmetry for the total row count: bindings=17 in the marker must
        # equal the number of `Binding {` struct literals in the table
        # (the same table the selftest pins at 17 reachable rows). Count
        # only the 4-space-indented struct literals inside the BINDINGS
        # const, not the `pub(crate) struct Binding {` definition line.
        self.assertEqual(
            len(re.findall(r"^    Binding \{", input_src, re.M)),
            17,
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

    def test_logout_window_open_gate_in_verify_step(self):
        # The wm.is_empty() no-op half of the Quit arm is pinned the same
        # way as the empty-desktop path: a dedicated PASS grep in the
        # Verify step plus the run_all wiring, AND a fast-fail FAIL line
        # (the window-open companion is outside the a11y PASS alternation,
        # so only its own greps cover it). A rename on either side fails
        # here before any QEMU boot can go green on a missing marker.
        block = _verify_block(_read(CI_YML))
        self.assertIn(
            "grep -qE \"\[test\] PASS test_logout_inert_with_window_open\"",
            block,
            "window-open logout grep missing from the Verify step",
        )
        self.assertIn(
            "ok &= input::test_logout_inert_with_window_open();",
            _read(RUN_ALL_RS),
            "test_logout_inert_with_window_open unwired from run_all - the "
            "CI grep can never pass",
        )
        line = ("[test] FAIL test_logout_inert_with_window_open: chord with window open "
        "(expected is_ending=false, got is_ending=true, exit_code=0)")
        self.assertTrue(
            self.gate.fail_re.search(line),
            "FAIL pattern %r does not match test_logout_inert_with_window_open "
            "(widen the alternation in ci.yml)" % self.fail_pat,
        )

    def test_input_suite_gates_in_verify_step(self):
        # Every input-contract test needs its own CI assertion, not just
        # the logout pair: test_keymap (routing table), test_from_raw
        # (packed decode), and test_session_end_gate (session-end gate
        # rules) are all input-suite tests OUTSIDE the a11y PASS
        # alternation, so the Verify step greps each PASS marker via the
        # shared for-loop, and the FAIL alternation names them for
        # fast-fail. Pin BOTH sides: the loop must exist in the Verify
        # block AND the tests must stay wired into run_all (a rename on
        # either side fails here before any QEMU boot can go green on a
        # missing marker).
        block = _verify_block(_read(CI_YML))
        self.assertIn('grep -qE "\[test\] PASS $t"', block)
        self.assertIn("for t in test_keymap test_from_raw test_session_end_gate", block)
        # On a missing PASS, the loop must ALSO grep the offending FAIL
        # line (like the a11y FAIL arm above) so the actual failure
        # message lands in CI logs, not just the tail.
        self.assertIn('grep -E "\[test\] FAIL $t" qemu_ade_log.txt || true', block)
        run_all = _read(RUN_ALL_RS)
        for name in ("test_keymap", "test_from_raw", "test_session_end_gate"):
            self.assertIn(
                "ok &= input::%s();" % name,
                run_all,
                "%s unwired from run_all - the CI grep can never pass" % name,
            )
            # The PASS prints themselves must exist in input.rs with
            # the exact test name -- otherwise a print-string rename
            # goes green host-side and red only in CI.
            self.assertIn("[test] PASS %s" % name, _read(INPUT_TESTS_RS))
            line = "[test] FAIL %s: something broke" % name
            self.assertTrue(
                self.gate.fail_re.search(line),
                "FAIL pattern %r does not match a failure of %s"
                % (self.fail_pat, name),
            )

    def test_esc_modal_legs_pinned_in_session_end_gate(self):
        # The Esc-Enter modal agreement on REAL input (Event::Key 0x1B
        # through handle_event, not the a11y activate path): Esc on an
        # empty desktop ends the session (0x1B is the byte-deliverable
        # logout key), while Esc with an overlay open closes the overlay
        # and must NOT end the session. Both legs live in
        # test_session_end_gate; the CI grep only proves the function
        # returns true, so a future edit that DELETES either leg goes
        # green in QEMU. Pin the legs' code markers + FAIL strings in
        # input.rs (RAW source - strip_rust would delete the print
        # strings) so removing either trips here before any boot.
        src = _read(INPUT_TESTS_RS)
        # Leg 1: Esc on an empty desktop -> is_ending() flips.
        self.assertIn("esc did not end empty session", src,
                      "empty-desktop Esc leg removed from test_session_end_gate")
        self.assertIn("Event::Key(keys::KEY_ESC as u16)", src)
        self.assertIn("if !d.session.is_ending() {", src)
        # Leg 2: Esc with the settings overlay open -> overlay closed,
        # session NOT ending (the dismiss half of the a11y arm).
        self.assertIn("esc with settings panel open ended session or kept it", src,
                      "settings-overlay Esc leg removed from test_session_end_gate")
        self.assertIn("d.settings.open = true;", src)
        self.assertIn("if d.session.is_ending() || d.settings.open {", src)
        # Leg 3: Esc with the switcher up confirms the selection and closes
        # the switcher (the keyboard modal is NOT in dismiss_overlays — a
        # real Esc would otherwise be a consumed no-op and the switcher
        # could never close). The a11y-arm branch in desktop.rs must stay in
        # lockstep with the contextual arm.
        self.assertIn("esc with switcher open ended session or kept it", src,
                      "switcher Esc leg removed from test_session_end_gate")
        self.assertIn("esc with switcher did not bring selection to front", src)
        self.assertIn("d.switcher_active = true;", src)
        desktop = _read(DESKTOP_RS)
        self.assertIn("if self.switcher_active {", desktop,
                      "a11y Esc arm lost the switcher branch")
        self.assertIn("self.switcher_active = false;", desktop)
        self.assertIn("self.wm.id_at(self.switcher_idx)", desktop)


    def test_terminal_esc_guard_pinned(self):
        # The Phase C terminal guard in the a11y Esc arm (desktop.rs): with
        # NO ring, NO overlay, NO fullscreen and a pty window focused, a
        # hardware Esc (0x1B) is forwarded to the shell instead of being
        # swallowed. The ring-active twin is the modality guard: first Esc
        # with the ring up dismisses only - 0x1B must NOT leak into the
        # shell, and the second Esc forwards. Both legs live in
        # test_session_end_gate (real Event::Key path); pin the mechanism
        # and the legs' FAIL strings so removing either trips host-side
        # before any QEMU boot.
        desktop = _read(DESKTOP_RS)
        # Mechanism: the arm captures ring state, and the guard gates BOTH
        # the pty write and the empty-desktop session-end on it (a ring-up
        # press may not end the session either - that is the two-Esc logout
        # contract).
        self.assertIn("let ring_was_active = self.focus_visible;", desktop,
                      "a11y Esc arm lost the ring capture")
        self.assertIn("if !ring_was_active {", desktop,
                      "a11y Esc arm lost the ring gate")
        self.assertIn("self.focused_has_pty()", desktop,
                      "a11y Esc arm lost the terminal guard")
        self.assertIn("libsarga::io::write(fd, &[keys::KEY_ESC])", desktop,
                      "a11y Esc arm lost the pty write")
        self.assertIn("self.wm.is_empty()", desktop,
                      "a11y Esc arm lost the empty-desktop session-end")
        src = _read(INPUT_TESTS_RS)
        # Leg 1: focused terminal -> Esc reaches the shell (slave reads
        # 0x1B), session NOT ending.
        self.assertIn("esc 0x1B did not reach shell", src,
                      "terminal-forward Esc leg removed from test_session_end_gate")
        self.assertIn("esc ended session with terminal focused", src)
        self.assertIn('AppWindow::new(100, 100, 400, 300, "EscTerm")', src)
        # Leg 2: ring-active Esc dismisses the ring only - no leak into the
        # shell; second Esc forwards.
        self.assertIn("ring-active esc leaked 0x", src,
                      "ring-active no-leak Esc leg removed from test_session_end_gate")
        self.assertIn("esc with ring on terminal ended session or kept ring", src)
        self.assertIn("esc after ring dismiss did not reach shell", src)
        self.assertIn('AppWindow::new(100, 100, 400, 300, "RingTerm")', src)


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

    def test_minimized_chrome_leaves_ring_pinned(self):
        # window::draw paints nothing for a minimized (non-animating) window
        # or a window pushed fully off-screen (x or y below -100) -- only
        # their shadow. The a11y tree mirrors that skip by marking the
        # Window/Close/Minimize nodes invisible, so the ring can't land on
        # undrawn chrome and the focused light silently miss (the same rule
        # as the taskbar overflow cap). Pin the build_tree gating + the
        # selftest legs (RAW source -- strip_rust would delete the print
        # strings), so a future edit that drops the mirror trips here
        # before any boot.
        m = _read(A11Y_MOD_RS)
        self.assertIn("surface the draw does not paint", m,
                      "minimized/off-screen chrome gating removed from build_tree")
        self.assertIn("WindowState::Minimized && aw.anim.is_none()", m,
                      "minimized skip no longer mirrors window::draw")
        self.assertIn("aw.x >= -100", m,
                      "off-screen x skip no longer mirrors window::draw")
        self.assertIn("aw.y >= -100", m,
                      "off-screen y skip no longer mirrors window::draw")
        self.assertIn("tree.set_visible(win_id, false)", m,
                      "Window node no longer hidden for undrawn windows")
        self.assertIn("tree.set_visible(close_id, false)", m,
                      "Close node no longer hidden for undrawn windows")
        self.assertIn("tree.set_visible(min_id, false)", m,
                      "Minimize node no longer hidden for undrawn windows")
        a11y = _read(A11Y_RS)
        self.assertIn('"minimized chrome leaves the ring"', a11y,
                      "minimized-chrome leg removed from test_a11y_taskbar_button")
        self.assertIn('"off-screen chrome leaves the ring"', a11y,
                      "off-screen-chrome leg removed from test_a11y_taskbar_button")
        self.assertIn("minimized window chrome still visible", a11y)
        self.assertIn("off-screen window chrome still visible", a11y)

    def test_taskbar_container_exclusion_pinned(self):
        # The Taskbar CONTAINER is a focusable fallback (no window -> the
        # ring lands on it), but it is not an interactive surface: the
        # resolver excludes it, so through the full focus_visible ->
        # RenderSnapshot::from pipeline snap.focused stays None (no phantom
        # light on the bar) while the ring stays parked on the container.
        # Pin the snapshot-level leg in test_a11y_taskbar_focus_feedback
        # (RAW source -- strip_rust would delete the print strings), so a
        # future edit that drops or weakens the exclusion trips here before
        # any boot.
        a11y = _read(A11Y_RS)
        self.assertIn("Taskbar container leaked a focus target", a11y,
                      "Taskbar-container exclusion leg removed")
        self.assertIn("Taskbar container focus was blurred", a11y,
                      "Taskbar-container leg lost its ring-parked assertion")
        self.assertIn('"Taskbar container excluded"', a11y,
                      "Taskbar-container PASS marker renamed or removed")
        self.assertIn("n.role == A11yRole::Taskbar", a11y,
                      "Taskbar-container leg no longer targets the Taskbar role")
        self.assertIn("d.focus_visible = true;", a11y)
        self.assertIn("RenderSnapshot::from(&d)", a11y,
                      "Taskbar-container leg no longer runs the snapshot pipeline")

    def test_activation_follows_window_pinned(self):
        # The ring follows the window a keyboard user just activated.
        # `activate_a11y_node` brings the window to front, REORDERING the
        # wm; node ids are positional, so the next rebuild renumbers every
        # window surface and `validate`'s fingerprint check would re-sync
        # the ring to a sibling taskbar button. `build_tree` consumes the
        # activation intent (`pending_window_focus`) to re-land the ring on
        # the activated window's OWN node. Pin both the mechanism and the
        # two selftest legs (raw source -- strip_rust would delete the print
        # strings), so a future edit that drops the intent or weakens the
        # legs trips here before any boot.
        a11y = _read(A11Y_RS)
        self.assertIn("ring follows activated window (not taskbar sibling)", a11y,
                      "taskbar-button activation leg removed from test_a11y_taskbar_button")
        self.assertIn("window-node activation keeps ring on window", a11y,
                      "window-node activation leg removed")
        self.assertIn("ring did not follow activated window A", a11y,
                      "taskbar-button leg FAIL string renamed")
        desktop = _read(DESKTOP_RS)
        self.assertIn("pending_window_focus: Option<WindowId>", desktop,
                      "pending_window_focus field removed from Desktop")
        self.assertIn("self.pending_window_focus = Some(wid);", desktop,
                      "activation no longer records the window-activation intent")
        mod = _read(A11Y_MOD_RS)
        self.assertIn("d.pending_window_focus.take()", mod,
                      "build_tree no longer consumes the activation intent")
        self.assertIn("n.owner == Some(wid) && n.role == A11yRole::Window", mod,
                      "build_tree no longer targets the activated window's own node")
        self.assertIn(
            """d.pending_window_focus.take() {
        if let Some(nid) = tree
            .nodes
            .iter()
            .find(|n| n.owner == Some(wid) && n.role == A11yRole::Window)
            .map(|n| n.id)
        {
            d.focus.focus(nid);""",
            mod,
            "build_tree no longer re-lands the ring on the activated window node")

    def test_close_animation_boundary_pinned(self):
        # The close-animation window: `wm.close(a)` shrinks the window for
        # ~8 ticks before `process_closing` removes it, and during that
        # window the tree STILL contains the closing window's chrome nodes
        # (same fingerprint), so the ring must follow the closing chrome —
        # `validate` keeps the focused id, it must NOT re-sync early — and
        # re-sync EXACTLY when the nodes vanish (the tick the anim
        # completes). Pin the leg markers + the boundary mechanism (RAW
        # source; strip_rust would delete the print strings), so a future
        # edit that drops the leg or moves the re-sync earlier/later trips
        # here before any boot.
        a11y = _read(A11Y_RS)
        self.assertIn('"close-anim fingerprint match follows, mismatch re-syncs"', a11y,
                      "close-animation window leg removed from "
                      "test_focus_validate_central")
        self.assertIn("close-anim ring left closing chrome", a11y,
                      "close-anim leg no longer asserts the ring follows the "
                      "closing chrome through the shrink")
        self.assertIn("close-anim did not land on sibling", a11y,
                      "close-anim leg no longer asserts the boundary lands "
                      "on the sibling window")
        self.assertIn("close-anim too short", a11y,
                      "close-anim leg no longer pins the shrink window length")
        # Both validate branches pinned at the real boundary: the identity
        # check must MATCH every shrink tick (ring kept on the closing
        # chrome) and MISMATCH at the vanish tick (ring re-synced). The
        # match/mismatch fingerprints are computed from the raw tree, so
        # they pin the branch conditions themselves, not just the outcome.
        self.assertIn("close-anim fingerprint changed mid-shrink", a11y,
                      "close-anim leg lost the MATCH-branch assertion "
                      "(fingerprint must stay stable through the shrink)")
        self.assertIn("close-anim boundary fingerprint still matched", a11y,
                      "close-anim leg lost the MISMATCH-branch assertion "
                      "(the vanish tick must change the focused id's "
                      "fingerprint)")
        # Boundary mechanism: the shrink-window counter and the follow-
        # through flag are unique to this leg (the mouse-close leg reuses
        # d.wm.close(a) and the same lookup idiom, so only the leg's own
        # identifiers trip here).
        self.assertIn("let mut shrink_ticks = 0u32;", a11y,
                      "close-anim leg lost the shrink-window counter")
        self.assertIn("ring_stayed_on_a = false;", a11y,
                      "close-anim leg lost the follow-through assertion")
        self.assertIn("fp_matched_every_shrink_tick", a11y,
                      "close-anim leg lost the fingerprint-match tracking "
                      "identifier")
        self.assertIn("boundary_old_fp", a11y,
                      "close-anim leg lost the pre-vanish fingerprint capture")

    def test_logout_protocol_fail_line_trips_fail_gate(self):
        # The generic a11y FAIL pattern must also catch a failure of the
        # logout protocol test (the FAIL gate greps test_ FAIL lines, not
        # just the a11y alternation), so a broken logout fails fast rather
        # than waiting for the PASS-count gate.
        # The FAIL gate pattern must now name the logout test: a broken
        # logout (is_ending not flipping, exit code drift, a near-miss key
        # corrupting the unwind) must fail fast via the FAIL grep, not wait
        # for the PASS-count gate.
        line = ("[test] FAIL test_logout_protocol_from_chord: chord near-miss corrupted the logout "
        "(expected is_ending=true, got is_ending=false, exit_code=0)")
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

    def test_missing_logout_pass_exits_one(self):
        # All a11y PASS lines AND the input-suite PASS lines present, the
        # aggregate verdict green -- but the logout-protocol PASS marker is
        # missing (unwired from run_all, renamed, or the chord test stopped
        # printing it). The count gate can NOT catch this (it only counts
        # the a11y alternation), so the dedicated grep is the teeth: mirror
        # the scenario a QEMU log would show and assert rc 1.
        lines = ["[test] PASS test_%s" % n for n in self.names]
        lines.append("[test] PASS test_logout_inert_with_window_open")
        lines.append("[test] PASS test_keymap")
        lines.append("[test] PASS test_from_raw")
        lines.append("[test] PASS test_session_end_gate")
        lines.append("[ade] selftest PASS")
        rc, msg = self.gate.verdict("\n".join(lines) + "\n")
        self.assertEqual(rc, 1, msg)
        # And the healthy twin (logout PASS restored) must pass -- proving
        # the missing-marker check, not something else, drove the failure.
        rc, msg = self.gate.verdict(healthy_log(self.names))
        self.assertEqual(rc, 0, msg)

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
        good = "[input] bindings=17 quit=1 chord=yes ctrlq=no grabs=10"
        log_line = "[ade] " + good  # the marker line as printed by ade
        self.assertIn(good, log_line)  # grep -qF would return 0 -> gate passes
        drifted = [
            "[input] bindings=17 quit=1 chord=yes ctrlq=no grabs=11",
            "[input] bindings=19 quit=1 chord=yes ctrlq=no grabs=10",
            "[input] bindings=17 quit=2 chord=yes ctrlq=no grabs=10",
            "[input] bindings=17 quit=1 chord=no ctrlq=no grabs=10",
            "[input] bindings=17 quit=1 chord=yes ctrlq=yes grabs=10",
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

    def test_focus_resolution_shares_focused_button_under_role(self):
        # The three focus-resolution computations (taskbar buttons by owner,
        # start-menu rows by bounds, window chrome by label) all share one
        # parent-role + focus-id lookup: focused_button_under_role in
        # a11y/mod.rs. This is a STRUCTURAL contract -- the extraction exists
        # so a future change to "is the focused Button under this parent
        # role?" has exactly one home. Re-inlining the lookup into any
        # family, or renaming/removing the helper, trips here before any
        # QEMU boot (the selftests only prove behavior, not the sharing).
        a11y = _read(A11Y_MOD_RS)
        desktop = _read(DESKTOP_RS)
        # The helper itself, defined once beside build_tree.
        self.assertIn("pub(crate) fn focused_button_under_role(", a11y,
                      "focused_button_under_role removed from a11y/mod.rs")
        self.assertIn("parent_role == role", a11y,
                      "helper no longer resolves by parent role")
        # focused_target routes all three families through it.
        self.assertIn("focused_button_under_role(&self.a11y_tree, fid, A11yRole::Taskbar)",
                      desktop, "Taskbar arm re-inlined its lookup")
        self.assertIn("focused_button_under_role(&self.a11y_tree, fid, A11yRole::StartMenu)",
                      desktop, "StartMenu arm re-inlined its lookup")
        self.assertIn("focused_button_under_role(&self.a11y_tree, fid, A11yRole::Window)",
                      desktop, "Window arm re-inlined its lookup")
        # The three index helpers reuse the same lookup: check the window
        # after each fn header rather than the whole file.
        for helper in ("menu_row_index", "menu_category_index", "menu_recent_index"):
            i = desktop.find("fn %s(" % helper)
            self.assertNotEqual(i, -1, "%s missing from desktop.rs" % helper)
            window = desktop[i:i + 700]
            self.assertIn("focused_button_under_role(", window,
                          "%s no longer routes through the shared helper" % helper)

    def test_tray_notification_focus_light_pinned(self):
        # Tray entries and notification rows are focusable a11y Buttons with
        # owner stamps, resolved in focused_target, and lit via the shared
        # window_button_face union / overlay focused param. This is the
        # STRUCTURAL contract of the keyboard-focus affordance: removing the
        # nodes, the resolution arms, or the draw wiring trips here before
        # any QEMU boot (the selftests only prove behavior, not the wiring).
        a11y = _read(A11Y_MOD_RS)
        node = _read(NODE_RS)
        window = _read(WINDOW_RS)
        desktop = _read(DESKTOP_RS)
        taskbar = _read(TASKBAR_RS)
        overlay = _read(NOTIF_OVERLAY_RS)
        render = _read(RENDER_MOD_RS)

        # Sentinels that name the two ownerless surfaces.
        self.assertIn(
            "pub(crate) const NOTIFICATION_OWNER: WindowId = WindowId(u64::MAX - 2);",
            window, "NOTIFICATION_OWNER sentinel removed from window.rs")
        self.assertIn(
            "pub(crate) const TRAY_PANEL_OWNER", window,
            "TRAY_PANEL_OWNER sentinel removed from window.rs")

        # The role enum: TrayPanel survives, the old Notification variant is
        # gone (rows are Buttons now, owner-stamped with the sentinel).
        self.assertIn("    TrayPanel,", node,
                      "TrayPanel role removed from A11yRole")
        self.assertNotIn("A11yRole::Notification", node,
                         "dead Notification role variant re-added")

        # build_tree emits one focusable Button per tray entry (owner-stamped,
        # bounds from the shared tray_entry_rect) and per visible notification
        # row (owner-stamped with the sentinel, bounds from notification_rect,
        # capped by NOTIF_MAX_VISIBLE).
        self.assertIn(
            "tree.set_owner(entry_id, crate::core::window::TRAY_PANEL_OWNER)",
            a11y, "tray entries lost their TRAY_PANEL_OWNER stamp in build_tree")
        self.assertIn("layout::tray_entry_rect(i, ty, d.screen_w, tray_len)",
                      a11y, "tray entry nodes no longer share tray_entry_rect")
        self.assertIn(
            "tree.set_owner(notif_id, crate::core::window::NOTIFICATION_OWNER)",
            a11y, "notification rows lost their NOTIFICATION_OWNER stamp")
        self.assertIn("layout::NOTIF_MAX_VISIBLE", a11y,
                      "notification node build dropped the visible cap")

        # focused_target resolves both families: a TrayPanel child by bounds
        # equality to HoverTarget::Tray(i); a Desktop child (only Button
        # children of Desktop are rows) by notification_rect to
        # HoverTarget::Notification(i).
        self.assertIn(
            "focused_button_under_role(&self.a11y_tree, fid, A11yRole::TrayPanel)",
            desktop, "Tray resolution arm re-inlined or removed")
        self.assertIn("HoverTarget::Tray(i)", desktop,
                      "tray focus no longer resolves to HoverTarget::Tray")
        self.assertIn(
            "focused_button_under_role(&self.a11y_tree, fid, A11yRole::Desktop)",
            desktop, "Notification resolution arm re-inlined or removed")
        self.assertIn("HoverTarget::Notification(i)", desktop,
                      "notification focus no longer resolves to Notification(i)")

        # Draw wiring: the tray draw routes the (hover, focused, mouse_down)
        # triple through the shared window_button_face union, the notification
        # overlay takes the focused param and lights hover==target ||
        # focused==target, and render/mod.rs passes snap.focused through.
        self.assertIn("window_button_face(", taskbar,
                      "tray draw no longer uses the shared button-face union")
        self.assertIn("snap.focused == Some(HoverTarget::Tray(i))", taskbar,
                      "tray draw no longer feeds focused into the union")
        # The tray entry draws the focused face as accent_light (the same
        # "blue = ring" rule as every other keyboard surface), so its exact
        # color choice is part of the same host-verifiable contract.
        self.assertIn("WindowButtonFace::Focused => th.accent_light,", taskbar,
                      "tray focused arm lost accent_light")
        self.assertIn("focused: Option<HoverTarget>", overlay,
                      "notification overlay lost the focused parameter")
        self.assertIn("focused_lit = focused == Some(target);", overlay,
                      "notification overlay no longer distinguishes the focused row")
        self.assertIn("theme.accent_light", overlay,
                      "notification overlay focused fill lost accent_light")
        # The overlay call passes both hover and focused (the two-line
        # sequence keeps the needle pinned to the draw_notifications call,
        # not the other draws that also pass snap.focused).
        i = render.find("draw_notifications")
        self.assertNotEqual(i, -1, "draw_notifications call removed from render/mod.rs")
        window_ = render[i:i + 220]
        self.assertIn("snap.hover,", window_,
                      "overlay call no longer passes snap.hover")
        self.assertIn("snap.focused,", window_,
                      "render/mod.rs no longer passes snap.focused to the overlay")

    def test_overflow_keyboard_wrap_pinned(self):
        # The overflow lockstep's keyboard-loop half: with more windows than
        # TASKBAR_MAX_BTNS the tree caps the taskbar buttons, so the ring on
        # the LAST visible button pressing Right must wrap to a LIVE node
        # (the first tray entry — the nearest focusable surface to the
        # right), not a phantom at the overflow "..." slot. A cap regression
        # would add the overflow window's button node, which would win the
        # spatial search as the nearest right-hand candidate and strand the
        # ring on an undrawn surface. Pin the leg's code markers + FAIL
        # string in a11y.rs (RAW source) so deleting or weakening it trips
        # here before any boot.
        a11y = _read(A11Y_RS)
        self.assertIn(
            "overflow keyboard wrap to live node",
            a11y,
            "overflow keyboard-wrap leg removed from test_a11y_taskbar_focus_feedback",
        )
        # The search must be seeded from the last VISIBLE button (index
        # TASKBAR_MAX_BTNS-1), not any button — a wrong seed would pass the
        # tray-entry landing without testing the overflow edge.
        self.assertIn(
            "layout::taskbar_btn_rect(crate::layout::TASKBAR_MAX_BTNS - 1, ty)",
            a11y,
            "overflow wrap leg no longer starts from the last visible button",
        )
        # The landed node must be the tray sentinel at the first tray-entry
        # rect — a node the draw actually paints.
        # The rect call also appears in the tray-panel test; anchor on the
        # wrap leg's own bounds-equality form so a leg-scoped edit trips.
        self.assertIn(
            "n.bounds == layout::tray_entry_rect(0, ty, d.screen_w, tray_len)",
            a11y,
            "overflow wrap leg no longer asserts the first tray entry",
        )
        self.assertIn(
            "Right from last visible button did not wrap to the first tray entry",
            a11y,
            "overflow wrap FAIL string renamed or removed",
        )
        # The leg runs on a WIDE desktop with icons cleared — the geometry
        # that makes the eight buttons on-screen and the tray entries the
        # only right-hand focusable surfaces (icons would pollute the
        # spatial search).
        # (1800, 700) and the icon clear also appear in other tests; pin the
        # leg's own setup window (overflow_win is leg-unique) instead.
        self.assertIn(
            "let mut d = Desktop::new(1800, 700);\n"
            "        d.desktop_icons.icons.clear();\n"
            "        let mut overflow_win = None;",
            a11y,
            "overflow wrap leg lost its wide-desktop/icon-clear setup",
        )

    def test_dispatch_sanity_job_pinned(self):
        # The dispatch-time self-check job: it runs ONLY on workflow_dispatch
        # (job-level `if`), asserts the trigger is still declared in the
        # checked-out ci.yml (grep '^  workflow_dispatch:'), and prints the
        # event payload for manual-run debugging. A future silent trigger
        # removal is caught twice: host-side here on every push, and in CI
        # at dispatch time by the job itself. Pin the job name, the guard,
        # the grep needle, and the PASS marker so an edit that renames or
        # weakens the job fails before any dispatch.
        ci = _read(CI_YML)
        self.assertIn("  dispatch-sanity:", ci,
                      "dispatch-sanity job renamed or removed from ci.yml")
        self.assertIn("    name: Dispatch sanity", ci,
                      "dispatch-sanity job lost its display name")
        self.assertIn("if: github.event_name == 'workflow_dispatch'", ci,
                      "dispatch-sanity job lost its workflow_dispatch-only guard")
        self.assertIn("grep -q '^  workflow_dispatch:'", ci,
                      "dispatch-sanity job lost its trigger-presence assertion")
        self.assertIn("PASS: workflow_dispatch trigger present", ci,
                      "dispatch-sanity PASS marker renamed or removed")
        self.assertIn('jq . "$GITHUB_EVENT_PATH"', ci,
                      "dispatch-sanity job lost its event-payload print")


if __name__ == "__main__":
    unittest.main()
