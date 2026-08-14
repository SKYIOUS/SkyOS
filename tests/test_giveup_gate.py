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
VAHID_RS = os.path.join(REPO_ROOT, "vahid", "src", "main.rs")
INIT_TOML = os.path.join(REPO_ROOT, "init", "Cargo.toml")


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
        cls.vahid = _read(VAHID_RS)

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

    def test_accounting_loop_is_single_flag_driven_deadline(self):
        # The probe + bounded + unbounded stages are ONE flag-driven loop
        # with a SINGLE deadline (Aug 12, 2026), so give-up-marker ordering
        # can never regress this gate the way the old three sequential
        # expects could. A future edit that reverts to sequential stages,
        # drops the saw_exit flag, or hard-exits inside the loop body is
        # caught here host-side, before any QEMU run.
        exp = self.exp
        # 1. Single deadline computed once, before the boot gate.
        self.assertIn("set account_deadline [expr {[clock seconds] + $timeout}]", exp)
        # 2. The one order-tolerant loop over that deadline.
        self.assertIn("while {!$deadline_hit}", exp)
        self.assertIn("set rem [expr {$account_deadline - [clock seconds]}]", exp)
        self.assertIn("expect -timeout [expr {$rem < 30 ? $rem : 30}]", exp)
        # 3. The accounting-live probe is a FLAG now, not an early-exit
        #    stage: its pass line must appear inside the loop body.
        self.assertIn("set saw_exit 0", exp)
        self.assertIn("PASS: init reaped a service exit (accounting live)", exp)
        # 4. The KERNEL-GATED verdict is decided AFTER the deadline from the
        #    flag (no exit observed), not by an early exit 0 mid-stream.
        self.assertIn("elseif {!$saw_exit}", exp)
        self.assertIn('send_user "KERNEL-GATED: init\'s waitpid reaps no exits - accounting markers deferred\\n"', exp)
        # 5. The old sequential stage timeouts are GONE (no expect -timeout
        #    60/30 stages with immediate exit 0).
        self.assertNotIn("expect -timeout 60 {", exp)
        self.assertNotIn("expect -timeout 30 {", exp)
        # 6. The loop body must not contain a premature exit 0 (the only
        #    exit 0s are the two post-loop verdicts).
        body_start = exp.index("while {!$deadline_hit}")
        # The loop body ends where the post-loop BOUNDED verdict comment
        # begins - spanning every expect arm. Anchoring on a marker
        # inside the body (e.g. 'set deadline_hit 1' in the top-of-loop
        # check) would truncate the span and let a premature 'exit 0' in
        # the arms slip past.
        body_end = exp.index("# 3'. BOUNDED verdict", body_start)
        self.assertNotIn("exit 0", exp[body_start:body_end])
        # 7. Structure order: deadline -> loop -> flag init -> adaptive
        #    timeout, then the post-loop verdicts.
        self.assertLess(
            exp.index("set account_deadline"), exp.index("while {!$deadline_hit}")
        )
        self.assertLess(
            exp.index("while {!$deadline_hit}"),
            exp.index("expect -timeout [expr {$rem < 30 ? $rem : 30}]"),
        )
        self.assertLess(
            exp.index("expect -timeout [expr {$rem < 30 ? $rem : 30}]"),
            exp.index("elseif {!$saw_exit}"),
        )

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

    def test_vahid_force_fail_hook_exists(self):
        # The CI fail-vahid ISO needs a REAL way to drive vahid's fatal /dev
        # path: vahid must accept --force-fail (argv, read via libsarga::args
        # like ade's --selftest), and init must pass it under the
        # force-vahid-fail feature. A future edit that renames or drops the
        # flag (or stops init from threading it) fails here host-side.
        self.assertIn('Some("--force-fail")', self.vahid)
        self.assertIn("[vahid] FORCED-FAIL:", self.vahid)
        self.assertIn("[vahid] FATAL: failed to create device nodes", self.vahid)
        self.assertIn("EXIT_DEVICE_SCAN_FAILED", self.vahid)
        # init side: the feature passes --force-fail as vahid's argv.
        self.assertIn('cfg!(feature = "force-vahid-fail")', self.init)
        self.assertIn('"--force-fail".to_string()', self.init)
        # And the feature is declared in init/Cargo.toml.
        self.assertIn("force-vahid-fail = []", _read(INIT_TOML))

    def test_ci_builds_and_boots_fail_vahid_iso(self):
        # The gate job must BUILD the fail-vahid ISO variant (init compiled
        # with the feature) and boot it through the give-up harness, so the
        # vahid give-up arm gets a real QEMU run, not a source-only pin.
        self.assertIn("force-vahid-fail", self.ci)
        self.assertIn("fail-vahid", self.ci)
        self.assertIn("qemu_giveup_boot.exp", self.ci)
        # Dedicated boot + verify: the fail-vahid ISO is booted as its OWN
        # step into its OWN log (qemu_failvahid_log.txt), and the verify
        # step asserts the vahid give-up marker specifically -- not the
        # shared qemu_giveup_log.txt, which stays the svc claim's.
        self.assertIn("skyos-ci-failvahid-", self.ci)
        self.assertIn("qemu_failvahid_log.txt", self.ci)
        # TTY0W-spanning grep (mirrors the svc verify's login-manager arm)
        # plus the specific [vahid] FATAL: marker, not a bare FATAL:.
        self.assertIn('giving up on [^\\n]*\\n?[^\\n]*vahid', self.ci)
        self.assertIn('grep -q "\\[vahid\\] FATAL:"', self.ci)
        # svc must be a ONE-SHOT on the fail-vahid build: its Usage exit is
        # far faster than vahid's forced /dev path, so respawning it would
        # race vahid for MAX_RESPAWNS and the boundedness wait could fire on
        # svc first, hiding the vahid give-up this step must prove.
        self.assertIn("respawn: !cfg!(feature = \"force-vahid-fail\")", self.init)

    def test_svc_claim_stays_on_the_healthy_iso(self):
        # Review fix: if the svc give-up boot used the fail-vahid ISO, BOTH
        # svc and vahid exit non-zero there and the harness's boundedness
        # wait would fire on whichever hits MAX_RESPAWNS first -- making the
        # 'giving up on svc' claim ambiguous (it might prove vahid's
        # boundedness instead). The svc boot must therefore EXCLUDE the
        # fail-vahid variant and keep its own log.
        self.assertIn("grep -v failvahid", self.ci)
        self.assertIn("qemu_giveup_log.txt", self.ci)
        # And the svc verify must NOT consume the fail-vahid log. Bound the
        # slice at the NEXT STEP's header -- the dedicated fail-vahid steps
        # are in the same job, so a job-level bound would still include them.
        start = self.ci.index("Verify bounded/unbounded")
        end = self.ci.index("- name: Boot fail-vahid ISO")
        block = self.ci[start:end]
        self.assertIn("qemu_giveup_log.txt", block)
        self.assertNotIn("qemu_failvahid_log.txt", block)

    def test_init_marker_sources_match_exp_patterns(self):
        # The markers the exp keys on must match init/src/main.rs's
        # write_all calls byte for byte (3-part writes + TTY0W interleave).
        self.assertIn('b"[init] service "', self.init)
        self.assertIn('b"[init] giving up on "', self.init)
        self.assertIn('b" after too many crashes\\n"', self.init)

    def test_exp_markers_match_init_three_part_writes(self):
        # Cross-pin (mirrors test_vahid_contract.py's marker cross-pin):
        # the give-up harness keys on the SAME three-part write markers
        # test_init_golden_trace.py parses (init/src/main.rs:151-153 and
        # :165-167 - prefix / service name / suffix as separate write_all
        # calls, with the kernel's [TTY0W] len=N diag interleaved between
        # them). The exp's escaped Tcl patterns must stay in lockstep with
        # those fragment boundaries AND with extract_event_stream's
        # interleave tolerance, so a marker rename or a fragment split
        # breaks here before the QEMU jobs go stale.
        exp = self.exp
        # 1. exit_live: "[init] service <name> exited" - three fragments,
        #    matched with (?s).*? to span the [TTY0W] interleave.
        self.assertIn(r"(?s)\[init\] service .*?exited", exp)
        self.assertIn('b"[init] service "', self.init)
        self.assertIn('b" exited\\n"', self.init)
        # 2. give_up: "[init] giving up on <name> after too many crashes".
        self.assertIn(r"(?s)\[init\] giving up on .*?svc", exp)
        self.assertIn(r"(?s)\[init\] giving up on .*?login-manager", exp)
        self.assertIn('b"[init] giving up on "', self.init)
        self.assertIn('b" after too many crashes\\n"', self.init)
        # 3. Lockstep with the golden-trace parser: extract_event_stream
        #    tolerates the interleaved newline after the [TTY0W] strip via
        #    (\n?), and the harness patterns span the same interleave with
        #    (?s). If the parser's tolerance changes but the exp's span
        #    does not (or vice versa), the two pin files drift - this is
        #    the tripwire.
        golden = _read(os.path.join(REPO_ROOT, "tests", "test_init_golden_trace.py"))
        self.assertIn("\\n?", golden)
        self.assertIn("TTY0W_RE", golden)
        # 4. The vahid give-up arm (forced-failure boot) uses the same
        #    marker shape as the svc arm, so the audit table stays honest.
        self.assertIn(r"(?s)\[init\] giving up on .*?vahid", exp)

    def test_exp_tracks_vahid_health_passively(self):
        # The healthy give-up boot must also prove vahid reached its
        # healthy sleep loop, tracked as a PASSIVE stage: '[vahid] ready'
        # sets saw_vahid and prints the phase verdict the CI Verify step
        # greps. The FATAL arm is deliberately NON-blocking (a note, no
        # exit 1) - on the KERNEL-GATED path vahid FATALs before ready can
        # print (pre-rewrite kernel, no devfs), and a fail-fast would break
        # the gate's deferral. An edit that turns the FATAL arm into a hard
        # exit (or drops the ready arm) fails here before any QEMU run.
        exp = self.exp
        self.assertIn("set saw_vahid 0", exp)
        self.assertIn("{\\[vahid\\] ready} {", exp)
        self.assertIn('send_user "PASS: vahid device manager healthy\\n"', exp)
        self.assertIn("set saw_vahid 1", exp)
        # FATAL arm is a note, not a fail-fast: no exit inside it.
        fatal_start = exp.index("{\\[vahid\\] FATAL:} {")
        fatal_end = exp.index("}", exp.index("NOTE: vahid device-manager FATAL"))
        self.assertNotIn("exit 1", exp[fatal_start:fatal_end])
        self.assertIn("NOTE: vahid device-manager FATAL", exp)

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
        # vahid health, two layers (gate-lifted only): the guest's raw
        # '[vahid] ready' marker AND the exp's phase verdict, so an exp
        # edit that drops the vahid stage can't hide behind the aggregate
        # bounded/unbounded PASS lines.
        self.assertIn('grep -q "\\[vahid\\] ready"', block)
        self.assertIn('grep -q "PASS: vahid device manager healthy"', block)

    def test_ci_verify_greps_node_table_and_pci_verdict(self):
        # The mknod contract must be exercised on real hardware: the Verify
        # step (gate-lifted only) greps the serial log for all six per-node
        # '[vahid] created /dev/<name>' markers - the exact names vahid's
        # node table carries (vahid/src/main.rs create_devices) - plus the
        # PCI scan verdict. An exp/verify edit that drops a node (or the
        # scan check) fails here before any QEMU run.
        block = self.ci[self.ci.index("Verify bounded/unbounded"):]
        self.assertIn(
            "for node in null zero random urandom tty console; do",
            block,
            "the six mknod-served node names drifted from the Verify grep loop",
        )
        self.assertIn('grep -q "\\[vahid\\] created /dev/$node"', block)
        self.assertIn("mknod table not exercised", block)
        # PCI scan verdict grep (scanned-N OR sysfs-unavailable).
        self.assertIn("scanned [0-9]+ PCI device", block)
        self.assertIn("sysfs unavailable", block)

    def test_host_tests_job_runs_this_pin(self):
        # This host test must be wired into CI's host-tests job.
        self.assertIn("test_giveup_gate.py", self.ci)

    def test_exp_captures_per_respawn_mem_series(self):
        # The per-respawn free-page series (Aug 12, 2026): login-manager
        # prints '[login] mem free=N pages' (ctlFS /ctl/sys/mem/free) before
        # EVERY Window::create (login-manager/src/main.rs), so on a
        # forced-failure boot (Window::create fails -> exit 0 -> unbounded
        # respawn) the stream carries one reading per respawn. The exp must
        # capture it LIVE into a list - the post-loop buffer grep would only
        # see the tail (match_max truncation) and lose the recovery
        # evidence that answers Option 1 vs 2.
        exp = self.exp
        # 1. Audit entry 6 documents the marker source + classification.
        #    Full row (name + marker): a bare-name pin would be satisfied by
        #    a renamed "# 6 mem_seriesX" prefix mutation.
        self.assertIn(r"#   6 mem_series   \[login\] mem free=(\d+) pages", exp)
        # 2. The live capture arm inside the accounting loop: escaped
        #    single-backslash Tcl regexp, numeric group, lappend, continue.
        self.assertIn(r"-re {\[login\] mem free=(\d+) pages} {", exp)
        self.assertIn("lappend mem_readings $expect_out(1,string)", exp)
        cap = exp.index("lappend mem_readings")
        self.assertIn("exp_continue", exp[cap:])
        # 3. The list is initialized before the loop (llength on an unset
        #    var is a Tcl error on the 0-readings path).
        self.assertIn("set mem_readings {}", exp)
        self.assertLess(
            exp.index("set mem_readings {}"), exp.index("while {!$deadline_hit}")
        )
        # 4. Classification vs kernel-gui-window-fix.md evidence table:
        #    recovering = transient -> Option 1; not = persistent -> Option 2.
        self.assertIn("mem_count == 0", exp)
        self.assertIn("mem_count == 1", exp)
        self.assertIn("(recovering = transient -> Option 1)", exp)
        # Strict >: a flat-at-zero series (0 -> 0) must classify persistent,
        # not transient - review edge (doc table keys on magnitude).
        self.assertIn("if {$mem_last > $mem_first} {", exp)
        self.assertIn("(not recovering = persistent -> Option 2)", exp)
        self.assertIn("free $mem_first -> $mem_last", exp)
        # 5. The 0-readings FAIL arm escapes [login] in the double-quoted
        #    send_user string (bracket-scan convention) and exits 1.
        self.assertIn(r"no '\[login\] mem free=' marker", exp)
        fail_start = exp.index("mem_count == 0")
        self.assertIn("exit 1", exp[fail_start:])
        # 6. The NOTE arm precomputes the value - inline [lindex ...] in a
        #    double-quoted string would trip test_no_unescaped_brackets.
        self.assertIn("set mem_first [lindex $mem_readings 0]", exp)
        self.assertIn("NOTE: mem series - single reading (free=$mem_first)", exp)

    def test_ci_verify_asserts_mem_series_verdict(self):
        # The Option 1 vs 2 table is now a CI-asserted contract, not an
        # ad-hoc reading: the healthy-boot Verify step requires a mem
        # verdict (NOTE = single reading / PASS = series) so a boot with no
        # marker fails; the fail-vahid Verify step requires the SERIES
        # verdict (>= 2 readings) on the forced-failure boot.
        ci = self.ci
        # 1. Healthy give-up verify (gate-lifted - after the KERNEL-GATED
        #    deferral exits): NOTE or PASS accepted.
        start = ci.index("Verify bounded/unbounded")
        end = ci.index("- name: Boot fail-vahid ISO")
        healthy = ci[start:end]
        self.assertIn('grep -qE "NOTE: mem series|PASS: mem series captured"', healthy)
        self.assertIn("marker contract broken on the healthy boot", healthy)
        # 2. Fail-vahid verify: the SERIES (>= 2 readings) is required -
        #    the per-respawn evidence that classifies transient vs
        #    persistent.
        fv = ci[ci.index("Verify vahid give-up on forced-failure boot"):]
        self.assertIn('grep -q "PASS: mem series captured"', fv)
        self.assertIn("Option 1 vs 2 evidence missing", fv)



if __name__ == "__main__":
    unittest.main()
