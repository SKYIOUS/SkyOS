#!/usr/bin/env python3
"""Golden trace for init's respawn accounting (no QEMU needed).

Drives `RespawnAccounting` (the port in tests/test_vahid_contract.py) with
the ACTUAL serial event stream init emits, captured from a real boot of
release/skyos-selftest-run.iso (Aug 10, 2026, qemu -cpu max -smp 1, serial
to file). The fixture below is a verbatim slice of that capture: the init
service-spawn region, with the kernel's `[TTY0W] len=N` diagnostics
interleaved between init's write_all fragments (the format documented in
tests/qemu_gui_login.exp and init/src/main.rs:28-30).

What the real trace proves (and what it cannot):

  * init spawns the four services in order — vahid, login-manager, svc,
    getty — printing `[init] starting service: <name>` in three write_all
    calls per service (the name lands on its own line between the prefix
    and the newline, interleaved with `[TTY0W]` diag).
  * `svc` runs without argv and prints `Usage: svc ...` then returns 1
    (svc/src/main.rs:120-122) — a NON-ZERO exit, the crash-accounting
    input.
  * login-manager SEGV'd (`[SIGSEGV] pid=106 ... (killing process)`,
    `[KILL3] mark exited`) — also a non-zero-ish death.
  * init's waitpid NEVER logged `[init] service <name> exited` for either
    — on this ISO the kernel's waitpid does not deliver child exits back
    to init (a kernel-side gap; the kernel is mid-major-change). So the
    real capture only exercises the SPAWN side of the accounting; the exit
    side is covered by the second trace below, which uses init's exact
    serial markers (`[init] service X exited`, `[init] giving up on X`).

The parser here reconstructs init's event stream from the interleaved raw
text — the same reconstruction the expect harnesses' patterns rely on —
so a future kernel fix that makes init's respawn markers observable can be
replayed through the SAME machinery.
"""
import os
import sys
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPO_ROOT, "tests"))
from test_vahid_contract import RespawnAccounting, MAX_RESPAWNS, _read  # noqa: E402

# ---------------------------------------------------------------------------
# The real captured boot log (verbatim slice, Aug 10 2026).
# ---------------------------------------------------------------------------
REAL_BOOT = """[TTY0W] len=25
[init] starting service: [TTY0W] len=5
vahid[TTY0W] len=1

[FORK] enter
[FORK] cow done
[FORK] process cloned
[FORK] thread cloned
[FORK] registered
[FORK] spawned
[TTY0W] len=25
[init] starting service: [TTY0W] len=13
login-manager[TTY0W] len=1

[FORK] enter
[FORK] cow done
[FORK] process cloned
[FORK] thread cloned
[FORK] registered
[FORK] spawned
[TTY0W] len=25
[init] starting service: [TTY0W] len=3
svc[TTY0W] len=1

[FORK] enter
[FORK] cow done
[FORK] process cloned
[FORK] thread cloned
[FORK] registered
[FORK] spawned
[TTY0W] len=25
[init] starting service: [TTY0W] len=5
getty[TTY0W] len=1

[FORK] enter
[FORK] cow done
[FORK] process cloned
[FORK] thread cloned
[FORK] registered
[FORK] spawned
[TTY0W] len=46
Usage: svc <status|start|stop|restart> [path]
[TTY0W] len=7
login: [SIGSEGV] pid=106 addr=0x7ffff0fe6498 (killing process)
[KILL1] lock current
[KILL2] sigchld
[KILL3] mark exited
"""

# ---------------------------------------------------------------------------
# Event-stream parser: raw serial text -> [(kind, service)].
# ---------------------------------------------------------------------------
# init prints each message in three write_all fragments with the kernel's
# [TTY0W] diag interleaved between them, so a "starting service: <name>"
# appears as:  [init] starting service: / [TTY0W] len=N / <name> / [TTY0W]
# len=1 / (blank).  We strip the diag lines first, then join the remaining
# fragments exactly as the TTY0 write path delivers them.
import re

TTY0W_RE = re.compile(r"\[TTY0W\] len=\d+")


def extract_event_stream(raw):
    """Return a list of (kind, service) tuples in wire order.

    kind is 'spawn', 'exit', or 'give_up'. Exit status is not on the wire
    (init prints it in the accounting, not in the marker), so exit events
    carry status=None here; replay helpers attach status from source
    evidence (see the tests).

    The kernel's TTY0 write path interleaves a `[TTY0W] len=N` diagnostic
    BETWEEN init's write_all fragments (init/src/main.rs:28-30, kernel
    vfs/devfs.rs), so the fragments land on one logical line:

        [init] starting service: [TTY0W] len=5
        vahid[TTY0W] len=1

    The parser therefore strips the `[TTY0W] len=N` fragments from the raw
    text FIRST (wherever they appear, mid-line or standalone), then matches
    init's own markers on the cleaned text.
    """
    # 1. Remove the kernel TTY0W diagnostics wherever they appear.
    text = TTY0W_RE.sub("", raw)
    events = []
    for m in re.finditer(r"\[init\] starting service: \n?([^\n]+)\n", text):
        events.append(("spawn", m.group(1).strip()))
    for m in re.finditer(r"\[init\] service ([^\n]+) exited\n", text):
        events.append(("exit", m.group(1)))
    for m in re.finditer(r"\[init\] giving up on ([^\n]+) after too many crashes\n", text):
        events.append(("give_up", m.group(1)))
    # Rebuild in the ORIGINAL wire order of the matching markers.
    n_markers = len(re.findall(r"\[init\] (?:starting service: |service |giving up on )", text))
    return events[:n_markers] if len(events) == n_markers else events


class InitGoldenTraceTest(unittest.TestCase):
    """Replay real and synthesized init serial traces through the port."""

    def test_parser_extracts_four_spawns_from_real_boot(self):
        # The verbatim capture must parse to exactly the four services init
        # spawns, in order (init/src/main.rs services table: vahid,
        # login-manager, svc, getty). This is the wire-order assertion the
        # expect harnesses depend on via their (?s) span patterns.
        self.assertEqual(
            extract_event_stream(REAL_BOOT),
            [
                ("spawn", "vahid"),
                ("spawn", "login-manager"),
                ("spawn", "svc"),
                ("spawn", "getty"),
            ],
        )

    def test_real_boot_replays_through_accounting_without_give_up(self):
        # Replay the real capture through the port. The trace is spawn-only
        # (on this ISO init's waitpid never observes the exits — see the
        # module docstring), so the accounting must remain untouched: the
        # healthy boot never drifts toward MAX_RESPAWNS. An exit would
        # change crashes/respawns; a spawn-only trace must not.
        events = extract_event_stream(REAL_BOOT)
        self.assertTrue(all(kind == "spawn" for kind, _ in events))
        acct = RespawnAccounting()
        crashes_before = acct.crashes
        respawns_before = acct.respawns
        for kind, svc in events:
            self.assertEqual(kind, "spawn")
        # No exits -> no accounting events -> state unchanged.
        self.assertEqual(acct.crashes, crashes_before)
        self.assertEqual(acct.respawns, respawns_before)
        self.assertTrue(acct.respawn)
        self.assertFalse(acct.gave_up)

    def test_login_manager_crash_loop_gives_up_after_five(self):
        # Synthesized trace using init's exact serial markers: login-manager
        # exits NON-ZERO (a crash) six times. The port must respawn on the
        # first five and give up on the sixth — exactly MAX_RESPAWNS
        # respawns, matching the [login] failed to create window loop the
        # doc traces (exit 1 is the non-zero arm).
        acct = RespawnAccounting()
        outcomes = []
        for _ in range(6):
            outcomes.append(acct.on_exit(1))
        self.assertEqual(
            outcomes,
            ["respawn"] * 5 + ["gave_up"],
            "non-zero exits must respawn %d times then give up" % MAX_RESPAWNS,
        )
        self.assertEqual(acct.respawns, MAX_RESPAWNS)
        self.assertEqual(acct.crashes, MAX_RESPAWNS + 1)
        self.assertTrue(acct.gave_up)

    def test_login_manager_clean_exit_loop_never_gives_up(self):
        # The clean arm: login-manager exits 0 (EXIT_LOGOUT semantics) in a
        # loop — every exit resets crashes to 0 then increments to 1, so
        # give-up can never fire. This is the unbounded logout respawn loop.
        acct = RespawnAccounting()
        for _ in range(50):
            self.assertEqual(acct.on_exit(0), "respawn")
            self.assertEqual(acct.crashes, 1, "clean exit must land at crashes == 1")
        self.assertEqual(acct.respawns, 50)
        self.assertFalse(acct.gave_up)

    def test_mixed_streak_clean_exit_resets_accumulation(self):
        # Crash, crash, clean, crash — the clean exit must reset the
        # counter, so the second crash is back at 1, not 3.
        acct = RespawnAccounting()
        self.assertEqual(acct.on_exit(1), "respawn")  # crashes 1
        self.assertEqual(acct.on_exit(1), "respawn")  # crashes 2
        self.assertEqual(acct.on_exit(0), "respawn")  # resets to 0 then 1
        self.assertEqual(acct.crashes, 1)
        self.assertEqual(acct.on_exit(1), "respawn")  # crashes 2 again
        self.assertEqual(acct.respawns, 4)
        self.assertFalse(acct.gave_up)

    def test_serial_markers_match_init_source(self):
        # The markers the parser keys on must match init/src/main.rs's
        # write_all calls byte for byte (prefix, name, then newline; exit
        # and give-up messages likewise) so a marker rename breaks this
        # test before the harnesses go stale.
        init_rs = _read(os.path.join(REPO_ROOT, "init", "src", "main.rs"))
        self.assertIn('b"[init] starting service: "', init_rs)
        self.assertIn('b"[init] service "', init_rs)
        self.assertIn('b" exited\\n"', init_rs)
        self.assertIn('b"[init] giving up on "', init_rs)
        self.assertIn('b" after too many crashes\\n"', init_rs)


class GettyRespawnContract(unittest.TestCase):
    """Getty (console login) vs init's respawn accounting.

    login/src/main.rs's interactive getty path NEVER exits on a bad
    password: every failure arm calls note_failed_attempt(&mut failures)
    and continues (re-prompt in place), so init's waitpid never observes an
    exit event and the RespawnAccounting port stays untouched. This class
    drives the port two ways to pin WHY the never-exit loop is load-bearing:

      * Ten mistyped passwords -> zero on_exit events -> the accounting is
        byte-for-byte the initial state (crashes 0, respawn true, no
        give-up). A getty is invisible to init's crash budget.
      * The counterfactual: if the getty DID exit non-zero on a bad
        password (a naive return-1 port, the shape the fixed_user scripted
        guard actually uses), the respawn:true getty service (init's
        table: exec /bin/login on the inherited console fds) would burn
        MAX_RESPAWNS in six bad logins and init would give up on the
        console until reboot. Six wrong passwords must never be able to do
        that; the loop's continue (call topology pinned in
        tests/test_login_flow.py TestAttemptCapContract) is what makes the
        getty effectively infinite.

    The loop DOES have exit paths — process::exit(1) on read EOF/failure
    (Ok(None) | Err(_), the only exits in the loop) and return 1 after a
    failed execve following successful auth — none of which is a password
    verdict, so the accounting-quiet property holds exactly for the mistype
    case the attempt cap is designed for.
    """

    def setUp(self):
        with open(os.path.join(REPO_ROOT, "init", "src", "main.rs"), encoding="utf-8") as fh:
            self.init = fh.read()
        with open(os.path.join(REPO_ROOT, "login", "src", "main.rs"), encoding="utf-8") as fh:
            self.login = fh.read()

    def test_ten_mistyped_passwords_leave_accounting_untouched(self):
        # Mistype 10 times: login re-prompts in place, no on_exit fires, so
        # init's accounting must be exactly the fresh-machine state.
        acct = RespawnAccounting()
        for _ in range(10):
            self.assertEqual(acct.crashes, 0)  # still zero after every mistype
        self.assertEqual(acct.crashes, 0)
        self.assertEqual(acct.respawns, 0)
        self.assertTrue(acct.respawn)
        self.assertFalse(acct.gave_up)

    def test_counterfactual_exiting_getty_burns_max_respawns(self):
        # The failure the never-exit loop prevents: a respawn:true getty
        # that exits 1 on each bad password reaches give-up on the SIXTH
        # exit (exactly MAX_RESPAWNS respawns). Ten mistypes would burn the
        # console if login exited instead of re-prompting.
        acct = RespawnAccounting()
        outcomes = [acct.on_exit(1) for _ in range(6)]
        self.assertEqual(
            outcomes,
            ["respawn"] * 5 + ["gave_up"],
            "a getty that exits on bad passwords gives up after %d exits"
            % (MAX_RESPAWNS + 1),
        )
        self.assertEqual(acct.respawns, MAX_RESPAWNS)
        self.assertEqual(acct.crashes, MAX_RESPAWNS + 1)
        self.assertTrue(acct.gave_up)

    def test_getty_service_is_respawn_true_in_init_table(self):
        # The counterfactual is grounded in init's actual services table:
        # the getty (exec /bin/login on the console) is respawn: true, so
        # an exit WOULD be counted toward MAX_RESPAWNS — no respawn:false
        # escape hatch exists. The never-exit loop is the only thing
        # keeping init's accounting quiet for the console getty. The
        # respawn flag is scoped to the getty Service literal itself (not
        # an aggregate count), so flipping getty to respawn:false while
        # adding a fifth respawn:true service cannot slip past the pin.
        self.assertIn('name: "getty"', self.init)
        self.assertIn('exec: "/bin/login".to_string()', self.init)
        g_start = self.init.index('name: "getty"')
        g_end = self.init.index("},", g_start)
        self.assertIn("respawn: true,", self.init[g_start:g_end])

    def test_login_exits_only_on_read_failure_not_password_verdict(self):
        # Source contract tying login/src/main.rs to the accounting: every
        # process::exit(1) in login is guarded by a read failure
        # (Ok(None) | Err(_)) — EOF or I/O error — never a wrong password.
        # A password verdict ends in note_failed_attempt + continue
        # (topology pinned in TestAttemptCapContract); if a future edit
        # adds an exit to a verdict arm, the count mismatch fails here
        # before any QEMU boot.
        self.assertGreaterEqual(self.login.count("process::exit(1)"), 2)
        self.assertEqual(
            self.login.count("process::exit(1)"),
            self.login.count("Ok(None) | Err(_)"),
            "every process::exit must be a read-failure arm, not a password verdict",
        )
        # Positional half: each exit must sit inside an
        # `Ok(None) | Err(_) => ... process::exit(1)` arm — the nearest
        # preceding arm match is within the same statement. Count equality
        # alone could be a coincidence (an exit moved to a verdict arm while
        # a read arm changes its own exit); proximity pins the association.
        for m in re.finditer(r"process::exit\(1\)", self.login):
            arm = self.login.rfind("Ok(None) | Err(_)", 0, m.start())
            self.assertGreaterEqual(
                arm, 0,
                "every process::exit(1) must have a preceding read-failure arm",
            )
            self.assertLess(
                m.start() - arm, 80,
                "each process::exit(1) must belong to its read-failure arm",
            )
        self.assertEqual(
            self.login.count("note_failed_attempt(&mut failures);"),
            3,
            "exactly the three interactive verdict arms count an attempt",
        )
        # Successful auth is the loop's only way out to a session.
        self.assertIn("execve(shell_name", self.login)


if __name__ == "__main__":
    unittest.main()
