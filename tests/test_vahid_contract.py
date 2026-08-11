#!/usr/bin/env python3
"""Host-runnable unit tests for vahid's exit-code discipline (no QEMU).

Pins the contract between vahid (vahid/src/main.rs) and init's respawn
accounting (init/src/main.rs): a FATAL device-scan failure must exit
NON-ZERO so init counts a crash and eventually gives up after
MAX_RESPAWNS, while a healthy vahid never exits (infinite sleep loop), so
init never sees an exit event and never respawns a working service.

The kernel-side respawn semantics (init/src/main.rs waitpid loop) are:

    status == 0  -> svc.crashes = 0  (clean exit resets the counter)
    svc.crashes += 1
    crashes > MAX_RESPAWNS -> give up (respawn = false)

So a service that exits 0 is treated as "ran its course" and respawns
forever; only a NON-ZERO exit accumulates crashes toward the give-up
threshold. vahid's discipline: exit(1) on fatal scan failure, never exit
on the healthy path. These tests assert the source contract so a future
refactor cannot silently flatten the exit code or drop the status lines.

The end-to-end semantics are pinned as a faithful port of init's
accounting (`RespawnAccounting`) — same order, same conditions — and
validated against the two real services: login-manager's exit(0)
window-failure loop is UNBOUNDED (give-up can never fire), while vahid's
exit(1) fatal path is BOUNDED (5 respawns, then give up).

Run:  python3 tests/test_vahid_contract.py
"""
import os
import re
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
VAHID_RS = os.path.join(REPO_ROOT, "vahid", "src", "main.rs")
INIT_RS = os.path.join(REPO_ROOT, "init", "src", "main.rs")

# Must match `const MAX_RESPAWNS: u32 = 5;` in init/src/main.rs — the
# source-contract test test_port_matches_source_max_respawns pins this.
MAX_RESPAWNS = 5


def _read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


class RespawnAccounting:
    """Faithful port of init/src/main.rs's waitpid-loop accounting.

    Mirrors the exact order and conditions of the source:

        if status == 0 { svc.crashes = 0; }   // reset FIRST
        if svc.respawn {
            svc.crashes += 1;
            if svc.crashes > MAX_RESPAWNS {    // strictly greater
                svc.respawn = false;           // give up (no spawn)
            } else {
                nanosleep(500ms); spawn();
            }
        }

    Consequences the tests rely on: a clean exit always lands at
    crashes == 1 (reset then +1), so give-up can never fire for an
    exit(0) service; a non-zero exit accumulates 1..6, respawning on
    1..5 and giving up on the 6th (exactly MAX_RESPAWNS respawns).

    Two source details are deliberately elided: the 500 ms nanosleep
    between respawns (wall-clock only, irrelevant to the counts) and
    pid tracking (init clears svc.pid on exit; the port models only the
    crash-counter state machine).
    """

    def __init__(self):
        self.crashes = 0
        self.respawn = True
        self.respawns = 0
        self.gave_up = False

    def on_exit(self, status):
        """One service-exit event. Returns 'respawn', 'gave_up', or
        'no_respawn' (respawn already disabled — post-give-up or a
        non-respawnable service; in init, svc.pid was cleared so no
        further exit events arrive anyway)."""
        if status == 0:
            self.crashes = 0
        if not self.respawn:
            return "no_respawn"
        self.crashes += 1
        if self.crashes > MAX_RESPAWNS:
            self.respawn = False
            self.gave_up = True
            return "gave_up"
        self.respawns += 1
        return "respawn"


class VahidExitCodeContract(unittest.TestCase):
    def setUp(self):
        self.vahid = _read(VAHID_RS)
        self.init = _read(INIT_RS)

    # --- vahid side: the exit-code + status-line contract ---

    def test_fatal_exit_code_is_non_zero(self):
        # The whole point: init only accumulates crashes (toward give-up)
        # on a NON-ZERO exit. A zero exit would reset the counter and
        # respawn a broken vahid forever.
        self.assertIn("const EXIT_DEVICE_SCAN_FAILED: i32 = 1;", self.vahid)
        self.assertIn("process::exit(EXIT_DEVICE_SCAN_FAILED)", self.vahid)

    def test_fatal_path_prints_status_line_before_exit(self):
        # The status line must be emitted before the exit so the serial log
        # records WHY vahid died (not just a bare exit code).
        fatal_line = self.vahid.index('[vahid] FATAL: failed to create device nodes')
        exit_call = self.vahid.index("process::exit(EXIT_DEVICE_SCAN_FAILED)")
        self.assertLess(fatal_line, exit_call)

    def test_healthy_path_never_exits(self):
        # After "[vahid] ready", the sleep loop must be the last thing —
        # no process::exit and no fallthrough return on the healthy path.
        ready = self.vahid.index("[vahid] ready")
        tail = self.vahid[ready:]
        self.assertNotIn("process::exit", tail)
        self.assertNotIn("return 0", tail)
        self.assertIn("loop {", tail)
        self.assertIn("nanosleep", tail)

    def test_scan_pci_reports_failure_instead_of_silent_skip(self):
        # scan_pci must return Option (Some(count) / None) so a missing
        # sysfs is an observable degraded state, not a swallowed error.
        self.assertIn("fn scan_pci() -> Option<usize>", self.vahid)
        self.assertIn("None", self.vahid)

    def test_create_devices_reports_success(self):
        # create_devices must return bool so the FATAL branch has a real
        # signal (previously it returned () and swallowed every error).
        self.assertIn("fn create_devices() -> bool", self.vahid)
        self.assertIn("all_ok", self.vahid)

    def test_status_markers_present(self):
        for marker in (
            "[vahid] SkyOS Device Manager",
            "[vahid] Scanning PCI...",
            "[vahid] ready",
            "[vahid] FATAL: failed to create device nodes",
        ):
            self.assertIn(marker, self.vahid, "missing marker: " + marker)

    def test_ready_marker_printed_exactly_once(self):
        # The healthy-path marker the harnesses grep ('[vahid] ready') is
        # vahid's terminal healthy state and MUST print exactly once: the
        # gate's state loop sets saw_vahid on the first occurrence, and a
        # second print would mask a marker-garbling regression behind the
        # duplicate. A future edit that adds an extra announce fails here
        # before any QEMU run.
        self.assertEqual(
            self.vahid.count("[vahid] ready"),
            1,
            "'[vahid] ready' must print exactly once on the healthy path",
        )

    def test_fatal_marker_printed_exactly_once_before_exit(self):
        # The fatal-path marker the gate greps ('[vahid] FATAL:') must print
        # exactly once, BEFORE the non-zero exit init's accounting counts
        # toward MAX_RESPAWNS. The order is the diagnostic contract: the
        # serial log must record WHY vahid died before the exit lands.
        fatal = self.vahid.index("[vahid] FATAL: failed to create device nodes")
        exit_call = self.vahid.index("process::exit(EXIT_DEVICE_SCAN_FAILED)")
        self.assertLess(fatal, exit_call)
        self.assertEqual(
            self.vahid.count("[vahid] FATAL:"),
            1,
            "'[vahid] FATAL:' must print exactly once on the fatal path",
        )

    def test_harness_grep_patterns_match_source_markers(self):
        # Cross-pin: the gate and shell harnesses grep these markers —
        # qemu_gui_gate.exp's state loop and qemu_shell_test.exp's
        # accumulated-buffer regexp. The escaped Tcl patterns must stay in
        # lockstep with the source strings, so a marker rename breaks here
        # before the QEMU jobs go stale.
        gate = _read(os.path.join(REPO_ROOT, "tests", "qemu_gui_gate.exp"))
        shell = _read(os.path.join(REPO_ROOT, "tests", "qemu_shell_test.exp"))
        self.assertIn(r"\[vahid\] ready", gate)
        self.assertIn(r"\[vahid\] FATAL:", gate)
        self.assertIn(r"\[vahid\] ready", shell)
        # The shell harness asserts HEALTHY vahid only; the fatal marker is
        # deliberately a GUI-gate concern. Adding it to the shell harness
        # must be a conscious change, not a silent drift.
        self.assertNotIn(r"\[vahid\] FATAL:", shell)

    def test_no_bogus_mknod_syscall(self):
        # Regression pin for the Aug 8, 2026 removal: the old code called
        # libsarga::syscall::syscall3(0x7d, ...) before the O_CREAT fallback,
        # but 0x7d is SYS_CLIPBOARD (125), not mknod — it could never create
        # a node and its result was discarded anyway. Node creation is the
        # O_CREAT fallback alone and drives all_ok -> the honest exit code.
        # A future edit that re-adds the discarded call fails here. (Pinned
        # on `syscall3`, not `0x7d`, because the removal-decision comment
        # legitimately mentions the number.)
        self.assertNotIn("syscall3", self.vahid)
        self.assertIn("open(&path, 0x41)", self.vahid)
        # Positive leg (Aug 10, 2026): pin the node table a future kernel
        # mknod must serve — exactly these six (name, major, minor) tuples
        # drive the loop, and the mknod contract in session-lifecycle.md
        # (SYS_MKNODAT=259 / SYS_MKNOD=133, dev = (major << 8) | minor)
        # round-trips them verbatim. Extracted from source, so a renamed,
        # added, or dropped node fails here before the doc target drifts.
        nodes = re.findall(r'\("(\w+)", (\d+), (\d+)\),', self.vahid)
        self.assertEqual(
            nodes,
            [
                ("null", "1", "3"),
                ("zero", "1", "5"),
                ("random", "1", "8"),
                ("urandom", "1", "9"),
                ("tty", "5", "0"),
                ("console", "5", "1"),
            ],
            "create_devices node table drifted from the six documented /dev nodes",
        )

    # --- init side: the accounting the exit code must feed ---

    def test_init_resets_crashes_on_clean_exit(self):
        # status == 0 resets crashes — a zero exit is therefore a forever-
        # respawn, which is why vahid's FATAL exit MUST be non-zero.
        self.assertIn("if status == 0 {", self.init)
        self.assertIn("svc.crashes = 0;", self.init)

    def test_init_accumulates_crashes_and_gives_up(self):
        self.assertIn("svc.crashes += 1;", self.init)
        self.assertIn("svc.crashes > MAX_RESPAWNS", self.init)
        self.assertIn("const MAX_RESPAWNS: u32 = 5;", self.init)

    # --- end-to-end semantics: faithful port of init's accounting ---

    def test_port_matches_source_max_respawns(self):
        # The port constant must not drift from the source it mirrors.
        self.assertIn(f"const MAX_RESPAWNS: u32 = {MAX_RESPAWNS};", self.init)

    def test_login_manager_clean_exit_loop_is_unbounded(self):
        # login-manager's '[login] failed to create window' path returns 0
        # (the exit(0) window-failure loop). Every clean exit resets crashes
        # BEFORE the increment, so crashes always lands at 1 and give-up can
        # never fire: UNBOUNDED. This is why MAX_RESPAWNS cannot kill the
        # GUI login on a bad window.
        sm = RespawnAccounting()
        for _ in range(1000):
            self.assertEqual(sm.on_exit(0), "respawn")
            self.assertFalse(sm.gave_up)
            self.assertEqual(sm.crashes, 1)  # reset to 0, then +1, every time

    def test_vahid_nonzero_exit_is_bounded(self):
        # vahid's FATAL path exits 1: crashes accumulate 1..6, respawning on
        # exits 1..5 and giving up on the 6th — exactly MAX_RESPAWNS
        # respawns, then dead. The serial log's '[init] giving up on vahid
        # after too many crashes' is the observable proof.
        sm = RespawnAccounting()
        exits = 0
        while not sm.gave_up and exits < 20:
            sm.on_exit(1)  # exits 1..5 respawn; exit 6 flips gave_up
            exits += 1
        self.assertTrue(sm.gave_up)
        self.assertEqual(exits, MAX_RESPAWNS + 1)   # 6 exits total
        self.assertEqual(sm.respawns, MAX_RESPAWNS)  # 5 respawns

    def test_mixed_streak_clean_exit_resets(self):
        # A bad streak followed by a clean exit resets the counter: give-up
        # is about the CURRENT streak, not lifetime (init's stated intent:
        # "a single bad streak doesn't permanently kill a service").
        sm = RespawnAccounting()
        for _ in range(4):
            sm.on_exit(1)  # crashes 1..4
        self.assertFalse(sm.gave_up)
        self.assertEqual(sm.on_exit(0), "respawn")
        self.assertEqual(sm.crashes, 1)  # reset to 0, then +1
        for _ in range(4):
            sm.on_exit(1)  # crashes 2..5
        self.assertFalse(sm.gave_up)
        self.assertEqual(sm.on_exit(1), "gave_up")  # crashes 6 > 5




    def test_init_resets_then_increments_order(self):
        # The reset-on-clean-exit MUST happen BEFORE the unconditional
        # increment. If these were swapped (increment then reset), a clean
        # exit would accumulate to MAX_RESPAWNS and eventually give up —
        # defeating the purpose of "a single bad streak doesn't permanently
        # kill a service". The source order is the invariant.
        code = self.init
        reset = code.index("svc.crashes = 0;")
        incr = code.index("svc.crashes += 1;")
        self.assertLess(
            reset, incr,
            "svc.crashes = 0 (reset) must come BEFORE svc.crashes += 1 "
            "(increment), so a clean exit never accumulates",
        )
        # The reset is inside `if status == 0 { ... }`, the increment is
        # inside `if svc.respawn { ... }`. The outer condition order is:
        # status check first, then respawn check. Swapping either the
        # conditions or the bodies would break the contract.
        status_idx = code.index("if status == 0 {")
        respawn_idx = code.index("if svc.respawn {")
        self.assertLess(status_idx, respawn_idx)

    def test_login_manager_window_failure_end_to_end(self):
        # Traces the full chain: login-manager's window-creation failure
        # return 0 -> init's waitpid sees status==0 -> crashes reset to 0
        # -> then +1 = 1 -> never reaches > MAX_RESPAWNS -> respawns forever.
        # This test reads login-manager source AND init source AND the
        # RespawnAccounting port and cross-checks all three.
        # Step 1: login-manager returns 0 on window-creation failure.
        lm_rs = os.path.join(REPO_ROOT, "login-manager", "src", "main.rs")
        with open(lm_rs, encoding="utf-8") as fh:
            lmsrc = fh.read()
        self.assertIn('io::print_str("[login] failed to create window\\n")', lmsrc)
        self.assertIn("return 0;", lmsrc[lmsrc.index("failed to create window"):])
        # Step 2: init's accounting resets on status==0 (not on status!=0).
        self.assertIn("if status == 0 {", self.init)
        self.assertIn("svc.crashes = 0;", self.init)
        # Step 3: the ported accounting (same order + conditions as init)
        # confirms unbounded respawn for exit(0): MAX_RESPAWNS can never fire.
        sm = RespawnAccounting()
        for _ in range(1000):
            self.assertEqual(sm.on_exit(0), "respawn")
            self.assertFalse(sm.gave_up)
        self.assertEqual(sm.crashes, 1)  # reset to 0, then +1, every time


if __name__ == "__main__":
    unittest.main(verbosity=2)
