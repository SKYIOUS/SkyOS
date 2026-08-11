#!/usr/bin/env python3
"""Host-runnable unit tests for the console login flow (no QEMU required).

Validates the initrd credential content built by build_initrd.py against the
exact parsing and verification semantics of the userspace login stack:

  /bin/login  (login/src/main.rs)
    lookup_user       -> parses /etc/passwd  (name:x:uid:gid:gecos:home:shell)
    verify_password   -> /etc/shadow via libsarga::hash::verify_password
                         (PBKDF2-HMAC-SHA256; salt hex after "PBKDF2-",
                          dk hex, optional ":<iterations>")

The shadow verifier in libsarga runs the PBKDF2 inside the kernel (SYS_HASH),
so it cannot execute on the host; this test ports its exact parse/verify shape
to hashlib and checks the *real* initrd constants (imported from build_initrd.py
so the test cannot drift), plus source-contract pins for login's execve argv[0]
change.

Run:  python3 tests/test_login_flow.py
"""
import hashlib
import os
import re
import sys
import unittest

from scan_rust import strip_rust

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, REPO_ROOT)
import build_initrd  # noqa: E402 -- real constants, must come after sys.path

LOGIN_RS = os.path.join(REPO_ROOT, "login", "src", "main.rs")
LOGIN_MANAGER_RS = os.path.join(REPO_ROOT, "login-manager", "src", "main.rs")
GUI_LOGIN_EXP = os.path.join(REPO_ROOT, "tests", "qemu_gui_login.exp")
LIBSARGA_LIB_RS = os.path.join(REPO_ROOT, "libsarga", "src", "lib.rs")
LIBSARGA_HASH_RS = os.path.join(REPO_ROOT, "libsarga", "src", "hash.rs")
INIT_RS = os.path.join(REPO_ROOT, "init", "src", "main.rs")
SESSION_RS = os.path.join(REPO_ROOT, "ade", "src", "service", "session.rs")
ADE_MAIN_RS = os.path.join(REPO_ROOT, "ade", "src", "main.rs")
CARGO_CONFIG = os.path.join(REPO_ROOT, ".cargo", "config.toml")
SARGA_JSON = os.path.join(REPO_ROOT, "x86_64-sarga.json")


def _hex(s):
    """hex::decode equivalent: None on invalid/odd input."""
    try:
        return bytes.fromhex(s.decode("ascii"))
    except (ValueError, UnicodeDecodeError):
        return None


def verify_password(shadow_data, username, password):
    """Byte-exact port of libsarga/src/hash.rs::verify_password.

    Accepts only PBKDF2-HMAC-SHA256 entries (other schemes are rejected for
    security); returns False for a missing user or a malformed entry.
    """
    for line in shadow_data.split(b"\n"):
        if not line:
            continue
        name, _, rest = line.partition(b":")
        if name != username.encode("utf-8"):
            continue
        if not rest.startswith(b"PBKDF2-"):
            return False
        salt_hex, _, rest3 = rest[len(b"PBKDF2-"):].partition(b":")
        salt = _hex(salt_hex)
        if salt is None or len(salt) != 16:
            return False
        if b":" in rest3:
            dk_hex, _, iter_str = rest3.partition(b":")
            try:
                iterations = int(iter_str.decode("ascii"))
            except (ValueError, UnicodeDecodeError):
                iterations = 10000
        else:
            dk_hex, iterations = rest3, 10000
            # NOTE (port divergence, crafted inputs only): Rust parses with
            # `iter_str.parse::<u32>().unwrap_or(10000)`; Python int() differs on
            # overflow ("99999999999999999999" -> Rust falls back to 10000) and
            # leading whitespace. Real shadow aging fields contain ':', so both
            # fall back to 10000; divergence is only reachable on crafted data.
        stored = _hex(dk_hex)
        if stored is None or len(stored) != 32:
            return False
        dk = hashlib.pbkdf2_hmac("sha256", password.encode("utf-8"), salt, iterations)
        return dk == stored
    return False


def lookup_shell(passwd_data, username):
    """Port of login/src/main.rs::lookup_user -- returns the shell field."""
    for line in passwd_data.split(b"\n"):
        if not line:
            continue
        parts = line.split(b":")
        if len(parts) == 7 and parts[0] == username.encode("utf-8"):
            return parts[6].decode("utf-8")
    return None


class TestLoginFlow(unittest.TestCase):
    SHADOW = build_initrd.SHADOW_CONTENT.encode("utf-8")
    PASSWD = build_initrd.PASSWD_CONTENT.encode("utf-8")

    # --- credential verification (the QEMU console login depends on this) ---
    def test_root_skyos_verifies(self):
        self.assertTrue(
            verify_password(self.SHADOW, "root", "skyos"),
            "root/skyos must verify against the initrd shadow",
        )

    def test_wrong_passwords_fail(self):
        for bad in ("root", "SKYOS", "skyos1", "skyos ", ""):
            with self.subTest(password=bad):
                self.assertFalse(verify_password(self.SHADOW, "root", bad))

    def test_unknown_user_fails(self):
        self.assertFalse(verify_password(self.SHADOW, "nobody", "skyos"))

    def test_non_pbkdf2_entry_rejected(self):
        self.assertFalse(verify_password(b"root:des-pw:hash\n", "root", "x"))
        self.assertFalse(verify_password(b"root:$6$salt$hash\n", "root", "x"))

    def test_malformed_entries_fail(self):
        self.assertFalse(verify_password(b"root:PBKDF2-0000\n", "root", "x"))
        self.assertFalse(
            verify_password(
                # (15-byte salt = the old SKYOSDESTOPSALT shape; rejected by the 16-byte
                # guard - doubles as a regression pin for the salt-length fix)
                b"root:PBKDF2-534b594f53444553544f5053414c54:zz\n", "root", "x"
            )
        )

    # --- the stored hash is literally PBKDF2("skyos", SKYOSDESKTOPSALT, 10000) ---
    # (16-byte salt is REQUIRED: libsarga verify_password enforces s.len() == 16)
    def test_stored_hash_is_pbkdf2_of_skyos(self):
        fields = self.SHADOW.strip().split(b":")
        self.assertEqual(fields[0], b"root")
        self.assertTrue(fields[1].startswith(b"PBKDF2-"))
        salt = bytes.fromhex(fields[1][len(b"PBKDF2-"):].decode("ascii"))
        self.assertEqual(salt, b"SKYOSDESKTOPSALT")
        stored = bytes.fromhex(fields[2].decode("ascii"))
        self.assertEqual(len(stored), 32)
        iterations = int(fields[3].decode("ascii"))
        self.assertEqual(iterations, 10000)
        self.assertEqual(
            hashlib.pbkdf2_hmac("sha256", b"skyos", salt, iterations), stored
        )

    # --- passwd -> shell (login execs this as argv[0]) ---
    def test_passwd_root_shell_is_sash(self):
        self.assertEqual(lookup_shell(self.PASSWD, "root"), "/bin/sash")

    def test_passwd_root_fields(self):
        parts = self.PASSWD.strip().split(b":")
        self.assertEqual(len(parts), 7)
        self.assertEqual((parts[2], parts[3]), (b"0", b"0"))  # uid, gid

    # --- source-contract pins for login's execve argv[0] change ---
    def test_login_execve_argv0_is_the_shell(self):
        with open(LOGIN_RS, encoding="utf-8") as fh:
            src = fh.read()
        # argv[0] must be the shell from /etc/passwd (the change that stopped
        # empty-argv argv/opt scans from misbehaving).
        self.assertIn("execve(shell_name, &[shell_name], &env_refs)", src)
        self.assertIn(
            'let shell_name = core::str::from_utf8(&_shell).unwrap_or("/bin/sash");',
            src,
        )

    def test_login_reads_etc_passwd_and_shadow(self):
        with open(LOGIN_RS, encoding="utf-8") as fh:
            src = fh.read()
        self.assertIn('const PASSWD_PATH: &str = "/etc/passwd";', src)
        self.assertIn('const SHADOW_PATH: &str = "/etc/shadow";', src)
        self.assertIn("verify_password(&username, &password)", src)

    # --- source-contract pins for login-manager's GUI failure path ---
    # A bad password must re-prompt in the window, never exit: the GUI session
    # is init service "login-manager" with respawn: true, so an auth-failure
    # exit would burn MAX_RESPAWNS the same way the console getty's would
    # have. (The console getty got the same treatment: it loops in place.)
    def test_gui_login_bad_password_reprompts_not_exits(self):
        with open(LOGIN_MANAGER_RS, encoding="utf-8") as fh:
            src = fh.read()
        # The Enter arm on verify failure must set the error + clear the
        # password buffer and fall through to re-render — no return/exit.
        self.assertIn('"Invalid username or password"', src)
        self.assertIn("password_buf.clear()", src)
        # Exit inventory (verified Aug 8, 2026) — exactly 3 `return`s:
        #   1. verify_password helper: `Err(_) => return false` (returns bool,
        #      not an exit from user_main)
        #   2. user_main: window-create failure -> `return 0`
        #   3. user_main: successful execve -> `return 0` (never returns)
        # plus no process::exit / panic! / .unwrap(), so NO auth-failure path
        # can exit user_main and burn init's MAX_RESPAWNS. Counting on
        # comment/string-stripped code catches `return 1`/`return -1`/`return
        # code` that a literal "return 0" search would miss, and comment/string
        # stripping stops innocent prose from false-tripping the count.
        code = strip_rust(src)
        self.assertEqual(
            len(re.findall(r"\breturn\b", code)),
            3,
            "login-manager exit inventory changed — update this pin deliberately",
        )
        self.assertNotIn("process::exit", src)  # covers libsarga::process::exit
        self.assertNotIn("panic!", code)
        self.assertNotIn(".unwrap()", code)



class TestAttemptCapContract(unittest.TestCase):
    """Source-contract pins for the getty attempt cap / backoff.

    login/src/main.rs throttles failed logins so a brute-forcer or stuck
    terminal cannot hammer the PBKDF2 verify (10k iterations each) at full
    speed, while the getty itself never exits (the MAX_RESPAWNS fix depends
    on that). These pins guard the constants and the exact call topology so
    a change to the throttle surfaces in CI before any QEMU boot.
    """

    @classmethod
    def setUpClass(cls):
        with open(LOGIN_RS, encoding="utf-8") as fh:
            cls.src = fh.read()
        cls.code = strip_rust(cls.src)

    def test_max_failed_attempts_is_10(self):
        self.assertIn("const MAX_FAILED_ATTEMPTS: u32 = 10;", self.src)

    def test_backoff_ns_is_30_seconds(self):
        # 30_000_000_000 ns == 30 s. The literal itself is the pin; the
        # arithmetic below just documents the unit conversion for the reader.
        self.assertIn("const BACKOFF_NS: u64 = 30_000_000_000;", self.src)
        self.assertEqual(30_000_000_000, 30 * 1_000_000_000)

    def test_note_failed_attempt_increments_then_resets_after_pause(self):
        # Counter increments; only disarmed when the pause actually happened
        # (EINTR must not skip the backoff and re-arm the next burst).
        self.assertIn("*failures += 1;", self.code)
        self.assertIn("if *failures >= MAX_FAILED_ATTEMPTS {", self.code)
        self.assertIn("Too many failed attempts - pausing 30s", self.src)
        self.assertIn("if io::nanosleep(BACKOFF_NS).is_ok() {", self.code)
        self.assertIn("*failures = 0;", self.code)

    def test_note_failed_attempt_called_on_exactly_three_paths(self):
        # Exactly the three interactive failure paths count an attempt:
        #   (1) unknown user, (2) invalid password encoding, (3) bad password.
        # The getty NEVER exits on these — it counts, then `continue`s.
        calls = re.findall(r"note_failed_attempt\(&mut failures\);", self.code)
        self.assertEqual(len(calls), 3, "note_failed_attempt call count changed")

    def test_three_paths_are_the_expected_ones(self):
        # Pin each failure path's message next to its call site: unknown user,
        # invalid encoding, bad password. The messages are multi-line Rust
        # strings (real newlines), so match them tolerantly inside print_str.
        for msg in ("login: unknown user", "Invalid password encoding", "Login incorrect"):
            self.assertRegex(self.src, r'print_str\(\s*"[\n]*' + re.escape(msg) + r'[\n]*"')
        # Every call is an INTERACTIVE-only path: guarded by the
        # fixed_user.is_some() early-return (scripted `login <user>` exits
        # without counting) and followed by `continue;` (re-prompt, never a
        # return). Both the guard and the re-prompt must sit with each call,
        # or getty-vs-scripted semantics silently change.
        code = self.code
        for _ in range(3):
            i = code.index("note_failed_attempt(&mut failures);")
            before = code[:i]
            # The guard immediately precedes the call (closing brace + guard).
            m = re.search(r"if fixed_user\.is_some\(\) \{\s*return 1;\s*\}\s*$", before)
            self.assertIsNotNone(
                m,
                "note_failed_attempt must be guarded by fixed_user.is_some() "
                "early-return (interactive-only path)",
            )
            tail = code[i + len("note_failed_attempt(&mut failures);"):]
            self.assertTrue(
                tail.lstrip().startswith("continue;"),
                "note_failed_attempt must be followed by continue; (re-prompt)",
            )
            code = tail
        # Exactly three interactive-only guards total (no stray early-returns).
        self.assertEqual(
            len(re.findall(r"if fixed_user\.is_some\(\) \{\s*return 1;", self.code)),
            3,
            "fixed_user.is_some() early-return count changed",
        )



    def test_fixed_user_guard_exits_non_zero(self):
        # The fixed_user (scripted `login <user>`) path exits with return 1
        # on every authentication failure — unknown user, invalid password
        # encoding, bad password. Exit code 1 means init's waitpid sees
        # status != 0, so crashes accumulate (svc.crashes += 1) and
        # eventually MAX_RESPAWNS gives up. If any guard used return 0
        # instead, init would reset crashes and respawn forever — masking
        # the failure. (Interactive getty: no argv, fixed_user is None, so
        # these guards are never reached; note_failed_attempt runs instead.)
        # Count on stripped code (self.code) so the regex is clean.
        guards = re.findall(
            r"if fixed_user\.is_some\(\) \{\s*return 1;\s*\}",
            self.code,
        )
        self.assertEqual(
            len(guards), 3,
            "must be exactly 3 fixed_user.is_some() -> return 1 guards "
            "(one per failure path: unknown user, invalid encoding, bad pw)",
        )
        # No alternative exit code (return 0) appears in any guard.
        self.assertNotIn("return 0;", self.code[self.code.index("fixed_user"):])

    def test_fixed_user_guard_before_note_failed_attempt(self):
        # Each `if fixed_user.is_some() { return 1; }` guard must appear
        # BEFORE its corresponding `note_failed_attempt(&mut failures);`
        # call — a scripted login exits without counting an attempt.
        # Check this on stripped code (self.code) so we compare against
        # the CALL SITE, not the function definition (which comes first).
        code = self.code
        for _ in range(3):
            i_note = code.index("note_failed_attempt(&mut failures);")
            before = code[:i_note]
            i_guard = before.rfind("if fixed_user.is_some()")
            self.assertGreaterEqual(
                i_guard, 0,
                "each note_failed_attempt call must be preceded by a "
                "fixed_user guard",
            )
            i_return = before.rfind("return 1;")
            self.assertGreaterEqual(
                i_return, 0,
                "each note_failed_attempt call must be preceded by return 1",
            )
            # The guard and return must come before the call.
            self.assertLess(i_guard, i_note)
            self.assertLess(i_return, i_note)
            code = code[i_note + len("note_failed_attempt(&mut failures);"):]

    def test_bare_enter_does_not_count(self):
        # A bare Enter re-prompts WITHOUT consuming an attempt: the empty-name
        # arm must `continue` without calling note_failed_attempt.
        code = self.code
        start = code.index("if name_bytes.is_empty() {")
        arm = code[start:]
        cut = arm.index("continue;")  # raises if the arm has no re-prompt
        self.assertNotIn("note_failed_attempt", arm[:cut])



class TestGuiAttemptCapContract(unittest.TestCase):
    """Source-contract pins for login-manager's GUI attempt cap / backoff.

    The GUI password field now has the same failed-attempt accounting as the
    console getty (login/src/main.rs): 10 bad logins -> a 30 s window message
    + serial announce + backoff, then re-prompt. The backoff runs AFTER the
    frame with the message is flushed (so it stays visible for the whole
    pause), and only disarms when the pause actually ran (EINTR must not
    skip it). A stray Enter with an empty username re-prompts without
    counting, matching the getty. The session NEVER exits on bad creds —
    init service "login-manager" has respawn: true, so an exit would burn
    MAX_RESPAWNS. These pins keep the throttle and the re-prompt semantics
    visible in CI before any QEMU boot.
    """

    @classmethod
    def setUpClass(cls):
        with open(LOGIN_MANAGER_RS, encoding="utf-8") as fh:
            cls.src = fh.read()
        with open(GUI_LOGIN_EXP, encoding="utf-8") as fh:
            cls.gui_login_exp = fh.read()
        cls.code = strip_rust(cls.src)

    def test_gui_max_failed_attempts_is_10(self):
        self.assertIn("const MAX_FAILED_ATTEMPTS: u32 = 10;", self.src)

    def test_gui_backoff_ns_is_30_seconds(self):
        self.assertIn("const BACKOFF_NS: u64 = 30_000_000_000;", self.src)

    def test_gui_note_failed_attempt_increments_then_resets_after_pause(self):
        self.assertIn("*failures += 1;", self.code)
        self.assertIn("if *failures >= MAX_FAILED_ATTEMPTS {", self.code)
        self.assertIn('*error_msg = String::from("Too many failed attempts - pausing 30s");', self.src)
        self.assertIn("io::nanosleep(BACKOFF_NS).is_ok() {", self.code)
        # The reset moved to the main-loop local (post-flush pause);
        # `failures = 0;` is a substring of both the old and new forms.
        self.assertIn("failures = 0;", self.code)

    def test_gui_note_failed_attempt_called_exactly_once_on_bad_creds(self):
        # The cap counts ONLY the bad-creds branch. The execve-failure path
        # after a SUCCESSFUL auth must NOT count (that is a correct login).
        calls = re.findall(r"note_failed_attempt\(&mut failures, &mut error_msg\);", self.code)
        self.assertEqual(len(calls), 1, "note_failed_attempt call count changed")
        # In the RAW source the call sits inside the bad-creds else branch:
        # the error message, then the password clear, then the call — as an
        # exact adjacent sequence (checked on src, not the string-masked code).
        self.assertIn(
            'error_msg = String::from("Invalid username or password");\n'
            "                        password_buf.clear();\n"
            "                        note_failed_attempt(&mut failures, &mut error_msg);",
            self.src,
        )
        # The execve-failure arm (after a successful auth) has no call: in
        # the raw source, the first closing brace after execve ends that arm.
        ev = self.src.index('process::execve("/bin/ade"')
        tail = self.src[ev:]
        cut = tail.index("}")
        self.assertNotIn("note_failed_attempt", tail[:cut])

    def test_gui_pause_sets_window_message_and_announces(self):
        self.assertIn("Too many failed attempts - pausing 30s", self.src)
        self.assertIn('io::print_str("\\nToo many failed attempts - pausing 30s\\n");', self.src)

    def test_gui_pause_runs_after_flush_so_message_shows(self):
        # The 30 s sleep must run AFTER win.flush(), so the pause message is
        # drawn before the window freezes — otherwise the user stares at a
        # stale frame for the whole backoff with no feedback.
        idx_flush = self.code.index("let _ = win.flush();")
        idx_sleep = self.code.index("io::nanosleep(BACKOFF_NS).is_ok() {")
        self.assertLess(idx_flush, idx_sleep)
        self.assertIn("failures >= MAX_FAILED_ATTEMPTS", self.code)

    def test_gui_empty_username_does_not_count(self):
        # Parity with the console getty's bare-Enter guard
        # (`if name_bytes.is_empty() { continue; }` in login/src/main.rs): a
        # stray Enter with no username re-prompts WITHOUT burning an attempt.
        code = self.code
        start = code.index("if user.is_empty() {")
        arm = code[start:]
        cut = arm.index("continue;")
        self.assertNotIn("note_failed_attempt", arm[:cut])
        self.assertIn("continue;", arm)

    def test_gui_bad_creds_announces_re_prompt_on_serial(self):
        # Parity with the console getty's "Login incorrect" pin: a bad GUI
        # password must print a serial announce so the QEMU harness
        # (tests/qemu_gui_login.exp) can assert the re-prompt on real
        # hardware - window stays up, no exit, no respawn. The announce sits
        # in the bad-creds else branch right after the note_failed_attempt
        # call (the adjacency pin anchors that branch), so the pinned
        # 3-line sequence is preserved.
        self.assertIn(
            'io::print_str("\\n[login] invalid credentials - re-prompting\\n");',
            self.src,
        )
        # Cross-source consistency: the harness must assert the same marker
        # (drift in either side fails this test).
        self.assertIn("invalid credentials - re-prompting", self.gui_login_exp)

    def test_gui_exit_inventory_still_three_returns_no_exit(self):
        # The throttle must not change the auth-failure exit behavior: still
        # exactly 3 returns (verify helper, window-create failure, successful
        # execve), no process::exit / panic! / .unwrap() — so NO auth-failure
        # path can exit user_main and burn init's MAX_RESPAWNS.
        code = strip_rust(self.src)
        self.assertEqual(len(re.findall(r"\breturn\b", code)), 3)
        self.assertNotIn("process::exit", self.src)
        self.assertNotIn("panic!", code)
        self.assertNotIn(".unwrap()", code)




    def test_gui_execve_error_keeps_window_alive(self):
        # When execve("/bin/ade") fails (binary missing or corrupt), the
        # window stays alive: the Err arm clears both buffers and falls
        # through to re-render with no return, no exit, no panic, and no
        # note_failed_attempt (a correct login is not a failure).
        code = strip_rust(self.src)
        # The serial announce proves this path is reached on real hardware.
        self.assertIn(
            'io::write_all(1, b"[login] execve failed, continuing\\n")',
            self.src,
        )
        # Both error_msg and password_buf are cleared before re-render.
        self.assertIn("error_msg.clear();", code)
        self.assertIn("password_buf.clear();", code)
        # The execve match body must not contain `return` (Err falls
        # through) or `note_failed_attempt` (correct login = not a failure).
        ev = code.index("process::execve(")
        tail = code[ev:]
        m_open = tail.index("{")          # match opening brace
        depth = 0
        match_end = None
        for i, ch in enumerate(tail[m_open:]):
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    match_end = m_open + i
                    break
        assert match_end is not None, "could not find execve match closing brace"
        err_arm = tail[m_open:match_end + 1]
        # The Err arm must contain no `return` (fallthrough to re-render)
        # and no `note_failed_attempt` (correct login is not a failure).
        err_start = err_arm.index("Err(_) => {")
        eb = err_arm[err_start:]
        depth = 0
        err_end = None
        for i, ch in enumerate(eb):
            if ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    err_end = i
                    break
        assert err_end is not None, "could not find Err arm closing brace"
        err_body = eb[:err_end + 1]
        self.assertNotIn("return", err_body)
        self.assertNotIn("note_failed_attempt", err_body)
        # The exit inventory must still be exactly 3 (this fix does not
        # introduce any new return path).
        self.assertEqual(len(re.findall(r"\breturn\b", code)), 3)
        self.assertNotIn("process::exit", self.src)
        self.assertNotIn("panic!", code)




    def test_gui_window_create_failure_returns_zero(self):
        # The window-creation failure arm returns 0. Init's waitpid sees
        # status == 0, resets crashes (svc.crashes = 0), then increments
        # (svc.crashes = 1). Result: crashes is always 1 — never reaches
        # MAX_RESPAWNS — so login-manager respawns forever even when the
        # window cannot be created. (Compare vahid's exit(1): crashes
        # accumulates to 6 and eventually gives up.)
        code = strip_rust(self.src)
        self.assertIn('io::print_str("[login] failed to create window\\n")', self.src)
        # The return 0 follows the failed-to-create print: in stripped code,
        # the Err arm is `Err(_) => { let _ = ...; return 0; }`.
        self.assertIn("return 0", code)
        # The exit inventory must still be exactly 3 (this is the same
        # window-failure return already counted by test_gui_exit_inventory).
        self.assertEqual(len(re.findall(r"\breturn\b", code)), 3)
        # Confirm the specific window-failure block is return 0 (not 1 or
        # process::exit). Find "failed to create window" then scan to next
        # "return" — it must be "return 0;".
        idx = self.src.index("failed to create window")
        tail = self.src[idx:]
        ret = tail.index("return") if "return" in tail else None
        self.assertIsNotNone(ret, "no return after failed to create window")
        self.assertTrue(tail[ret:].lstrip().startswith("return 0;"))



class TestPanicContract(unittest.TestCase):
    """Panic contract for the auth binaries (login, login-manager).

    Traced Aug 10, 2026: libsarga/src/lib.rs defines the ONLY panic handler
    in the userspace stack (both login and login-manager link libsarga):

        #[panic_handler]
        fn panic(info: &core::panic::PanicInfo) -> ! {
            crate::println!("SARGA OS PANIC: {}", info);
            process::exit(1);
        }

    and sarga_main! is a thin wrapper:

        pub extern "Rust" fn main() -> i32 { $main_fn() }

    — no catch, no unwind. The build forces -C panic=abort
    (.cargo/config.toml, x86_64-sarga.json), so core::panic::catch_unwind
    is not usable: a panic reaches the handler and calls process::exit(1),
    which init's waitpid sees as a NON-ZERO status. That accumulates
    svc.crashes and burns a MAX_RESPAWNS respawn exactly like a bad exit
    code. Therefore the auth paths must be TOTAL (never panic) — a hash
    failure must return false and re-prompt, never exit(1) via the panic
    handler.

    KNOWN BOUNDARY: the credential logic is panic-free (no .unwrap(),
    no panic!, no unchecked slicing), but both auth binaries allocate on
    the auth path (login's read_whole_file / read_line Vec growth,
    login-manager's read_to_string / String::from / buffer push). An
    allocation failure routes through libsarga's #[alloc_error_handler]
    (mem.rs:150), which panics and exits 1 — the one vector the source scans cannot see. This is accepted as a
    genuine system-level OOM (not a credential failure): a corrupt or
    oversized shadow on an out-of-memory box burns a respawn the same way
    any OOM would. If that ever becomes reachable, the fix is a hard cap
    on the shadow read, not a catch_unwind.

    These pins guard the contract: the handler stays exit(1) (a panic is
    a crash, never a silent exit 0), the build stays panic=abort (so
    nobody can rely on catch_unwind), the auth/hash code stays panic-free
    in its credential logic, and the OOM boundary is pinned explicitly so
    it cannot be silently forgotten.
    """

    @classmethod
    def setUpClass(cls):
        with open(LIBSARGA_LIB_RS, encoding="utf-8") as fh:
            cls.lib = fh.read()
        with open(LIBSARGA_HASH_RS, encoding="utf-8") as fh:
            cls.hash = fh.read()
        with open(LOGIN_RS, encoding="utf-8") as fh:
            cls.login = fh.read()
        with open(LOGIN_MANAGER_RS, encoding="utf-8") as fh:
            cls.gui = fh.read()
        cls.lib_code = strip_rust(cls.lib)
        cls.hash_code = strip_rust(cls.hash)
        cls.login_code = strip_rust(cls.login)
        cls.gui_code = strip_rust(cls.gui)

    def test_panic_handler_exits_non_zero(self):
        # A panic must exit NON-ZERO: init's waitpid sees status != 0 and
        # accumulates crashes. If someone changed this to exit(0), a panic
        # would reset crashes and respawn forever — hiding the bug — so the
        # exit(1) is itself the contract.
        self.assertIn("#[panic_handler]", self.lib)
        self.assertIn("fn panic(", self.lib)
        self.assertIn("process::exit(1);", self.lib_code)
        self.assertNotIn("process::exit(0);", self.lib_code)

    def test_build_is_panic_abort(self):
        # panic=abort everywhere: catch_unwind is impossible, so the ONLY
        # defense against a panic burning a respawn is making the auth code
        # total. A future switch to panic=unwind would still need a custom
        # hook (catch_unwind + a fallback path); until that exists, the
        # abort strategy is pinned.
        with open(CARGO_CONFIG, encoding="utf-8") as fh:
            cfg = fh.read()
        with open(SARGA_JSON, encoding="utf-8") as fh:
            target = fh.read()
        self.assertIn("panic=abort", cfg)
        self.assertIn('"panic-strategy": "abort"', target)

    def test_alloc_error_handler_is_known_oom_boundary(self):
        # The ONLY panic vector the credential-logic scans cannot see is
        # allocation failure: Vec growth in read_whole_file / read_line
        # hits libsarga's #[alloc_error_handler], which panics and exits
        # 1 (mem.rs:150: "allocation error"). Pin that boundary explicitly
        # so it stays visible — an OOM is a system-level failure (accepted),
        # not a silent exit 0 that would hide the bug.
        with open(os.path.join(REPO_ROOT, "libsarga", "src", "mem.rs"), encoding="utf-8") as fh:
            mem = fh.read()
        # alloc_error_handler panics -> the panic handler (lib.rs, pinned by
        # test_panic_handler_exits_non_zero) turns that into exit(1).
        self.assertIn("#[alloc_error_handler]", mem)
        self.assertIn("allocation error", mem)
        self.assertIn("panic!", strip_rust(mem))

    def test_sarga_main_is_thin_no_catch(self):
        # sarga_main! is `fn main() -> i32 { $main_fn() }` — no try/catch,
        # no conversion. A panic propagates straight to the panic handler.
        self.assertIn("pub extern \"Rust\" fn main() -> i32", self.lib)
        self.assertIn("$main_fn()", self.lib)
        self.assertNotIn("catch_unwind", self.lib)

    def test_hash_verify_path_is_total(self):
        # libsarga/src/hash.rs verify_password must never panic: every
        # fallible step is unwrap_or / early-return-false. A malformed
        # shadow entry (bad hex, wrong salt length, non-PBKDF2 scheme)
        # must produce `false`, not a panic -> exit(1) -> respawn burn.
        self.assertIn("fn verify_password(", self.hash)
        self.assertNotIn(".unwrap()", self.hash_code)
        self.assertNotIn(".expect(", self.hash_code)
        self.assertNotIn("panic!", self.hash_code)
        # The one slice into shadow data (`rest[7..]` after starts_with
        # b"PBKDF2-" and `rest3[..pos]` after position()) is length-
        # guarded by construction: starts_with implies len >= 7, position()
        # returns a valid index. Pin that both guards sit with the slices.
        self.assertIn('rest.starts_with(b"PBKDF2-")', self.hash)
        self.assertIn("let rest2 = &rest[7..];", self.hash)
        self.assertIn(".position(|&b| b == b':')", self.hash)
        self.assertIn("dk_hex = &rest3[..pos];", self.hash)

    def test_login_auth_path_has_no_panic_vectors(self):
        # The console getty's auth path (verify_password wrapper, read_line,
        # read_whole_file, note_failed_attempt) must be panic-free so a
        # shadow/hash failure re-prompts instead of exiting via panic.
        self.assertNotIn(".unwrap()", self.login_code)
        self.assertNotIn(".expect(", self.login_code)
        self.assertNotIn("panic!", self.login_code)
        # The only slice in login is `&tmp[..n]` in read_whole_file, where
        # n comes from read(fd, &mut tmp) — the kernel guarantees n <= 512,
        # so it is safe by construction (not a panic vector). Pin that it
        # stays a read-buffer slice and no hand-indexed slice appears.
        self.assertIn("&tmp[..n]", self.login)
        # No hand-computed slice indexes (a `&x[expr..]` where expr is not
        # a read-count) anywhere in login — those would be panic vectors.
        self.assertNotIn("[7..]", self.login)
        self.assertNotIn("[pos", self.login)
        self.assertNotIn("[..pos", self.login)

    def test_login_manager_auth_path_has_no_panic_vectors(self):
        # The GUI login's auth path must be panic-free for the same reason:
        # a bad creds or execve failure re-renders; it must never exit via
        # the panic handler's process::exit(1).
        self.assertNotIn(".unwrap()", self.gui_code)
        self.assertNotIn(".expect(", self.gui_code)
        self.assertNotIn("panic!", self.gui_code)
        # login-manager's existing pins already assert no process::exit /
        # panic! / .unwrap() on the raw source; this pin covers the
        # stripped-code view so comments/strings can't hide a new panic.


def _parse_bindings(input_code):
    """Extract the BINDINGS rows from stripped ade/src/input/mod.rs as dicts."""
    block = input_code.split("const BINDINGS:")[1].split("];")[0]
    rows = []

    def field(chunk, name):
        m = re.search(r"\b" + name + r":\s*([^,]+),", chunk)
        return m.group(1).strip() if m else None

    for chunk in block.split("Binding {"):
        if "code:" not in chunk:
            continue
        rows.append({
            "code": field(chunk, "code"),
            "ctrl": field(chunk, "ctrl") == "true",
            "alt": field(chunk, "alt") == "true",
            "shift": field(chunk, "shift") == "true",
            "desktop": field(chunk, "desktop") == "true",
            "action": field(chunk, "action"),
        })
    return rows


class TestKernelKeyContract(unittest.TestCase):
    """Pins the Phase C packed-key contract while the kernel rewrite is in flight.

    Source: ade/docs/kernel-gui-modifier-delivery.md, Design A. The kernel is
    mid-major-change, so this test holds the *userspace half* — libsarga
    get_key() -> Option<u16> (A5) and ade KeyEvent::from_raw (A6) — plus a
    Python port of the decode, so the libsarga/input changes cannot drift
    while the kernel side (A1-A4) is being rewritten.

    Bit layout (pinned): low byte = char; bit8 = alt, bit9 = ctrl,
    bit10 = shift, bit11 = super (ignored by ade). The chord
    Ctrl+Alt+Backspace = 0x08 | (1<<8) | (1<<9) = 0x0308.
    The same values are pinned at boot by the ade selftest test_from_raw.
    """

    @classmethod
    def setUpClass(cls):
        cls.gui = open(os.path.join(REPO_ROOT, "libsarga", "src", "gui.rs"), encoding="utf-8").read()
        cls.input = open(os.path.join(REPO_ROOT, "ade", "src", "input", "mod.rs"), encoding="utf-8").read()
        cls.event = open(os.path.join(REPO_ROOT, "ade", "src", "core", "event.rs"), encoding="utf-8").read()
        cls.desktop = open(os.path.join(REPO_ROOT, "ade", "src", "core", "desktop.rs"), encoding="utf-8").read()
        cls.main = open(os.path.join(REPO_ROOT, "ade", "src", "main.rs"), encoding="utf-8").read()
        cls.input_code = strip_rust(cls.input)
        cls.desktop_code = strip_rust(cls.desktop)

    # --- Python port of the decode (the contract mirror) -----------------
    @staticmethod
    def from_byte(b):
        """Mirror of ade/src/input/mod.rs KeyEvent::from_byte."""
        if b == 0x1B:
            return (0x1B, False, False, False)        # Esc
        if b in (0x0D, 0x0A, 28):
            return (0x0D, False, False, False)        # Enter
        if b == 0x09:
            return (0x09, False, False, False)        # Tab
        if b in (0x7F, 0x08):
            return (0x7F, False, False, False)        # Backspace
        if 1 <= b <= 26:
            return (ord("a") - 1 + b, True, False, False)  # Ctrl+letter
        return (b, False, False, False)

    @staticmethod
    def from_raw(raw):
        """Mirror of ade/src/input/mod.rs KeyEvent::from_raw (Design A)."""
        code, ctrl, alt, shift = TestKernelKeyContract.from_byte(raw & 0xFF)
        if raw & (1 << 8):
            alt = True
        if raw & (1 << 9):
            ctrl = True
        if raw & (1 << 10):
            shift = True
        return (code, ctrl, alt, shift)

    def test_libsarga_get_key_is_u16(self):
        # A5 landed: the producer no longer truncates to u8; the u64 syscall
        # result widens losslessly to u16 so the modifier bits can arrive.
        self.assertIn("pub fn get_key(&mut self) -> Option<u16>", self.gui)
        self.assertIn("Some(k as u16)", self.gui)
        self.assertNotIn("Some(k as u8)", self.gui)

    def test_ade_from_raw_exists_with_bit_layout(self):
        # A6 landed: low byte decoded via from_byte, bits 8/9/10 OR'd in.
        self.assertIn("pub fn from_raw(raw: u16) -> KeyEvent", self.input_code)
        self.assertIn("(raw & 0xFF) as u8", self.input_code)
        for mask in ("1 << 8", "1 << 9", "1 << 10"):
            self.assertIn(mask, self.input_code, f"missing {mask}")

    def test_event_key_payload_is_u16_and_wired(self):
        # The packed value must reach handle_key un-truncated.
        self.assertIn("Key(u16)", self.event)
        self.assertIn("desktop_win.get_key()", self.main)
        self.assertIn("Event::Key(key)", self.main)

    def test_desktop_routes_via_from_raw_and_guards_a11y(self):
        # handle_key decodes the packed value; the pty path sends the low byte.
        self.assertIn("KeyEvent::from_raw(key)", self.desktop_code)
        self.assertIn("(key & 0xFF) as u8", self.desktop_code)
        # The a11y pre-handler must skip modified events so the chord (0x308)
        # falls through to the keymap instead of being swallowed.
        self.assertIn("key & 0xFF00 != 0", self.desktop_code)

    def test_python_mirror_spec_values(self):
        table = [
            (0x0063, (ord("c"), False, False, False)),    # plain 'c'
            (0x000D, (0x0D, False, False, False)),        # Enter
            (0x0003, (ord("c"), True, False, False)),     # Ctrl+C fold
            (0x0308, (0x7F, True, True, False)),          # chord: backspace+ctrl+alt
            (0x0108, (0x7F, False, True, False)),         # bit8 = alt (alt-only)
            (0x0208, (0x7F, True, False, False)),         # bit9 = ctrl (ctrl-only)
            (0x0808, (0x7F, False, False, False)),        # bit11 super ignored
            (0x0103, (ord("c"), True, True, False)),      # Ctrl+C + alt bit
        ]
        for raw, expected in table:
            self.assertEqual(self.from_raw(raw), expected, f"from_raw(0x{raw:04x})")

    def test_inert_until_kernel_sends_bits(self):
        # Zero high bits: from_raw must be byte-identical to from_byte —
        # the property that makes the u16 path safe to land now.
        for b in range(256):
            self.assertEqual(self.from_raw(b), self.from_byte(b), f"byte 0x{b:02x}")

    def test_chord_resolves_to_quit_in_table(self):
        rows = _parse_bindings(self.input_code)
        quit_rows = [r for r in rows if r["action"] == "KeyAction::Quit"]
        self.assertEqual(len(quit_rows), 1, "exactly one Quit binding")
        q = quit_rows[0]
        self.assertEqual(q["code"], "keys::KEY_BACKSPACE")
        self.assertTrue(q["ctrl"] and q["alt"] and not q["shift"] and q["desktop"])
        # The packed chord decodes to exactly that row's (code, mods) tuple.
        code, ctrl, alt, shift = self.from_raw(0x0308)
        self.assertEqual((code, ctrl, alt, shift), (0x7F, True, True, False))
        # Ctrl+Q must stay unbound: neither plain 'q' nor Ctrl+Q matches a
        # Quit row (the old session-end gates are gone).
        self.assertFalse(any(r["code"] in ("b'q'", "keys::KEY_Q") for r in rows))

    def test_from_raw_wired_into_run_all(self):
        # Boot coverage of the pin must not silently vanish: the selftest
        # runs only if run_all calls it, and the [input] serial marker
        # covers only the keymap dump, not from_raw.
        mod_rs = strip_rust(
            open(os.path.join(REPO_ROOT, "ade", "src", "util", "testing", "mod.rs"), encoding="utf-8").read()
        )
        self.assertIn("ok &= input::test_from_raw();", mod_rs)


class TestScancodeConstants(unittest.TestCase):
    """Pins the userspace set-1 scancode constants against the standard.

    Values sourced from pc-keyboard 0.5.1's ScancodeSet1
    (src/scancodes.rs, `map_scancode` / `map_extended_scancode`): Escape=0x01,
    Enter=0x1C, F11=0x57, and the arrow keys as the SECOND byte of the E0
    extended sequence (ArrowUp=E0 48, ArrowLeft=E0 4B, ArrowRight=E0 4D,
    ArrowDown=E0 50) — see ade/docs/kernel-keyboard-gate.md §2.1. The
    constants are what the future RawKey path would push into the byte
    stream, so a typo here (e.g. SCAN_UP=71) would break a11y arrow
    navigation / F11 only at boot — this pin catches it host-side.
    """

    INPUT_RS = os.path.join(REPO_ROOT, "ade", "src", "input", "mod.rs")

    EXPECTED = {
        "SCAN_ESC": 0x01,     # set-1 Escape make
        "SCAN_ENTER": 0x1C,   # set-1 Enter make
        "SCAN_UP": 0x48,      # E0 48 (ArrowUp)
        "SCAN_LEFT": 0x4B,    # E0 4B (ArrowLeft)
        "SCAN_RIGHT": 0x4D,   # E0 4D (ArrowRight)
        "SCAN_DOWN": 0x50,    # E0 50 (ArrowDown)
        "SCAN_F11": 0x57,     # set-1 F11 make
    }

    @classmethod
    def setUpClass(cls):
        cls.input_code = strip_rust(open(cls.INPUT_RS, encoding="utf-8").read())

    def test_constant_values_match_set1(self):
        for name, expected in self.EXPECTED.items():
            m = re.search(
                r"pub const %s: u8 = (0x[0-9A-Fa-f]+|[0-9]+);" % name,
                self.input_code,
            )
            self.assertIsNotNone(m, f"{name} missing from input/mod.rs keys module")
            self.assertEqual(
                int(m.group(1), 0),
                expected,
                f"{name} drifted from its set-1 scancode value (see docs/kernel-keyboard-gate.md §2.1)",
            )

    def test_arrow_constants_are_e0_extended(self):
        # The arrows are E0-extended keys: their values are the E0 second
        # bytes, which collide with the single-byte numpad block
        # (0x47..=0x53). This is a latent collision (numpad keys decode to
        # Unicode digits or RawKey(Numpad*) and never reach the a11y byte
        # path today), pinned here so nobody "fixes" SCAN_UP=0x48 to a
        # numpad-adjacent value.
        e0 = {self.EXPECTED[k] for k in ("SCAN_UP", "SCAN_LEFT", "SCAN_RIGHT", "SCAN_DOWN")}
        self.assertTrue(e0 <= set(range(0x47, 0x54)), "arrow constants must sit in the E0 second-byte range")


class TestPhaseBRoutingHarness(unittest.TestCase):
    """Pins the sendkey routing probes added to the QEMU harnesses + CI gates.

    The Phase B open question (kernel-keyboard-gate.md section 6) is answered
    on every CI run via the one-shot [KBD] IRQ1 fired! marker; these pins keep
    the probes wired so a future edit cannot silently delete them. Also pins
    the Tcl bracket-escaping fix: an unescaped [KBD] inside a double-quoted
    send_user string is Tcl command substitution and would crash the harness
    with "invalid command name KBD".
    """

    @classmethod
    def setUpClass(cls):
        cls.login = open(os.path.join(REPO_ROOT, "tests", "qemu_gui_login.exp"), encoding="utf-8").read()
        cls.gate = open(os.path.join(REPO_ROOT, "tests", "qemu_gui_gate.exp"), encoding="utf-8").read()
        cls.ci = open(os.path.join(REPO_ROOT, ".github", "workflows", "ci.yml"), encoding="utf-8").read()

    def test_login_harness_has_chord_and_irq_probe(self):
        self.assertIn("proc sendkey_chord {mods key}", self.login)
        self.assertIn("sendkey_chord {ctrl alt} backspace", self.login)
        self.assertIn("proc monitor_cmd {cmd}", self.login)

    def test_gate_harness_has_irq_probe(self):
        self.assertIn("proc monitor_cmd {cmd}", self.gate)
        self.assertIn('monitor_cmd "sendkey shift"', self.gate)

    def test_ci_verify_step_has_explicit_vahid_assertions(self):
        # The gate Verify step must assert vahid positively - both the
        # guest marker ('[vahid] ready') and the exp's phase PASS line
        # ('PASS: vahid device manager healthy') - not just the aggregate
        # verdict string. The guest marker alone cannot catch an exp edit
        # that drops the vahid phase (the marker is real guest output and
        # is present either way); only the exp PASS line proves the phase
        # ran. Dropping either grep must fail this host test.
        anchor = self.ci.index("Verify GUI gate verdict")
        # Slice at the next step boundary so the pin covers exactly this
        # step - a future edit that moves the greps into a later step (the
        # give-up Verify step follows) must fail here, not pass because the
        # greps are still somewhere downstream in the file.
        nxt = self.ci.find("\n      - name: ", anchor)
        block = self.ci[anchor:nxt] if nxt != -1 else self.ci[anchor:]
        self.assertIn('grep -q "\\[vahid\\] ready"', block)
        self.assertIn('grep -q "PASS: vahid device manager healthy"', block)

    def test_gate_exp_emits_vahid_phase_verdict(self):
        # Lockstep with the ci.yml grep #2: the gate exp must actually emit
        # the exact 'PASS: vahid device manager healthy' verdict line the
        # Verify step greps for. If the exp's verdict string ever changes,
        # every healthy boot fails the gate with a misleading message - this
        # host pin catches the drift first (same pattern as
        # test_vahid_contract's marker cross-pin).
        self.assertIn('send_user "PASS: vahid device manager healthy', self.gate)

    def test_bracket_escaping_in_tcl_strings(self):
        # Unescaped [KBD] in a double-quoted Tcl string = command substitution
        # = "invalid command name KBD" crash at runtime.
        self.assertIn('send_user "=== routing probe: \[KBD\] IRQ1 fired! ===\\n"', self.login)
        self.assertIn('send_user "=== routing probe: \[KBD\] IRQ1 fired! ===\\n"', self.gate)
        self.assertNotIn('send_user "=== routing probe: [KBD]', self.login)
        self.assertNotIn('send_user "=== routing probe: [KBD]', self.gate)



    def test_no_unescaped_brackets_in_executable_strings(self):
        # Scan-based companion to test_bracket_escaping_in_tcl_strings: EVERY
        # double-quoted Tcl string on an executable (non-comment) line must
        # have its brackets escaped. '[' inside "..." is Tcl command
        # substitution ("invalid command name X" crash), while '{...}' is
        # substitution-free, so only the double-quoted form is scanned.
        import re

        def unescaped_bracket_lines(text):
            hits = []
            for i, line in enumerate(text.splitlines(), 1):
                if line.lstrip().startswith("#"):
                    continue
                for m in re.finditer(r'"(?:[^"\\]|\\.)*"', line):
                    body = m.group(0)[1:-1]
                    j = 0
                    while j < len(body):
                        if body[j] == "\\":
                            j += 2
                            continue
                        if body[j] == "[":
                            hits.append(i)
                            break
                        j += 1
            return hits

        # Scan EVERY Tcl harness in tests/ (login/gate today; any new .exp
        # added later is covered automatically). Verified clean Aug 10, 2026:
        # qemu_shell_test.exp and qemu_ade_selftest.exp have zero hits; the
        # three login FAIL arms were the last survivors.
        import glob
        import os

        exp_dir = os.path.join(REPO_ROOT, "tests")
        for path in sorted(glob.glob(os.path.join(exp_dir, "*.exp"))):
            content = open(path, encoding="utf-8").read()
            name = os.path.basename(path)
            hits = unescaped_bracket_lines(content)
            self.assertEqual(
                hits, [],
                f"{name}: unescaped '[' in double-quoted executable string(s) "
                "at line(s) {hits} — Tcl command substitution crash",
            )

    def test_second_session_leg_present(self):
        # Step 7: after init respawns login-manager (which execs ade directly
        # on the GUI path - there is NO getty 'login:' prompt on respawn),
        # the harness waits for the SECOND '[ade] session established' marker
        # so the negative leg runs on a live desktop. This pins the marker
        # wait so a future edit can't silently drop the leg.
        self.assertIn('send_user "=== second session ===\\n"', self.login)
        self.assertIn("PASS: second session established", self.login)
        # No getty login may appear in the leg: the second session comes from
        # login-manager's exec of ade, not a console login.
        self.assertNotIn("PASS: second login prompt", self.login)

    def test_keyboard_window_open_chain_present(self):
        # Step 8: the ONLY path that opens a window on today's kernel
        # (arrows are E0-dropped, Ctrl+letter folds don't arrive) is the
        # a11y chain Tab (ring on Taskbar) -> Enter (start menu) -> type
        # "term" -> Enter (launch). The harness must drive exactly that
        # sequence and wait for the positive [ade] launched Terminal marker
        # (desktop.rs launch_app) before the negative probe.
        self.assertIn('sendkey_seq "tab"', self.login)
        self.assertIn('sendkey_seq "term"', self.login)
        self.assertIn('\\[ade\\] launched Terminal', self.login)
        self.assertIn("PASS: window opened via keyboard", self.login)

    def test_negative_leg_chord_with_window_open(self):
        # Step 9: with a window open, the true ctrl+alt+backspace chord must
        # NOT end the session (wm.is_empty() guard holds on real input, the
        # QEMU counterpart of test_session_end_gate's synthetic window case).
        # The leg must FAIL on the marker, not accept it.
        leg = self.login[self.login.index("negative leg: chord with window open"):]
        self.assertIn("sendkey_chord {ctrl alt} backspace", leg)
        self.assertIn("FAIL: chord ended the session with a window open!", leg)
        self.assertIn("guard_holds - chord did not end session", leg)
        self.assertIn("PASS: guard_holds", leg)
        # The leg sits BEFORE the final PASS banner, so the suite verdict
        # only prints if the negative probe held.
        self.assertLess(
            self.login.index("negative leg: chord with window open"),
            self.login.index("GUI login integration: PASS"),
        )

    def test_ci_a11y_gate_includes_keyboard_window_open(self):
        # The selftests test_a11y_keyboard_window_open,
        # test_a11y_start_menu_rows and test_a11y_overlay_mouse_keyboard_parity
        # must be counted in the ci.yml PASS-name gate (15 names now); the
        # gate test (test_ade_selftest_gate.py) already pins set-equality
        # with run_all.
        self.assertIn("a11y_keyboard_window_open", self.ci)
        self.assertIn("a11y_start_menu_rows", self.ci)
        self.assertIn("a11y_overlay_mouse_keyboard_parity", self.ci)
        self.assertIn('"$a11y_passes" -lt 18', self.ci)
        self.assertIn("found $a11y_passes/18", self.ci)

    def test_ci_verify_steps_grep_irq_marker_after_verdict(self):
        # Both Verify steps grep the marker (escaped for BRE) and only after
        # the verdict PASS check, so an early boot death reports the verdict
        # failure rather than a misleading routing diagnosis.
        self.assertEqual(self.ci.count(r"\[KBD\] IRQ1 fired!"), 2, "both Verify steps grep the marker")
        self.assertEqual(self.ci.count(r'! grep -q "\[KBD\] IRQ1 fired!"'), 2)
        pass_idx = self.ci.find("GUI login integration: PASS")
        irq_idx = self.ci.find(r"\[KBD\] IRQ1 fired!", pass_idx)
        self.assertGreater(irq_idx, pass_idx, "login IRQ1 grep must come after the PASS check")


class TestInitRespawnContract(unittest.TestCase):
    """Pins the session-end side of init's respawn accounting.

    ade's logout contract (EXIT_LOGOUT = 0, ade/src/service/session.rs) only
    holds because init's waitpid loop (init/src/main.rs) resets a service's
    crash counter on a CLEAN exit and accumulates crashes only toward
    MAX_RESPAWNS. These pins hold BOTH sides so a change to either cannot
    silently break the logout loop (session end -> exit 0 -> init sees a
    clean exit -> respawns login-manager -> ade again).

    Source contract (verified Aug 10, 2026, init/src/main.rs:132-145):

        // Clean exit (status == 0) means the service ran its course ...
        if status == 0 { svc.crashes = 0; }        // reset FIRST
        if svc.respawn {
            svc.crashes += 1;
            if svc.crashes > MAX_RESPAWNS {        // strictly greater
                svc.respawn = false;               // give up
            } else { nanosleep(500ms); spawn(); }
        }

    Consequences the pins rely on: a clean exit always lands at
    crashes == 1 (reset then +1), so give-up can never fire for an
    exit(0) service; only a non-zero (crash) exit accumulates toward the
    threshold. session.rs's EXIT_LOGOUT comment states the same contract
    in ade's terms ("init resets its crash counter and respawns the login
    service; non-zero = crash (init counts it toward MAX_RESPAWNS)") --
    these tests assert that cross-source agreement so the two sides can't
    drift apart.

    NOTE: init's view of an exit is BINARY (status == 0 vs everything
    else). It does not reuse session.rs's finer-grained exit_class
    (0 = clean, 1..=127 = exit code, 128+sig = killed by signal): a
    signal-killed service still has a non-zero waitpid status, so it is
    counted toward MAX_RESPAWNS just like a bad exit code. That matches
    the "non-zero (or signal) exits count" framing -- there is no status
    init treats as a crash that is not non-zero.
    """

    @classmethod
    def setUpClass(cls):
        cls.init = open(INIT_RS, encoding="utf-8").read()
        cls.init_code = strip_rust(cls.init)
        cls.session = open(SESSION_RS, encoding="utf-8").read()
        cls.session_code = strip_rust(cls.session)
        cls.ade_main = open(ADE_MAIN_RS, encoding="utf-8").read()
        cls.ade_main_code = strip_rust(cls.ade_main)

    # --- init side (init/src/main.rs) ---
    def test_max_respawns_constant_is_5(self):
        # Deliberately pinned here too (not just in test_vahid_contract.py,
        # which ports the same state machine): this file holds the SESSION
        # side of the contract, and a MAX_RESPAWNS bump must fail loudly in
        # both places so neither side of the agreement drifts silently.
        self.assertIn("const MAX_RESPAWNS: u32 = 5;", self.init)

    def test_zero_status_resets_crash_counter_first(self):
        # The reset must run BEFORE the increment: a clean exit always
        # lands at crashes == 1, so give-up can never fire for an exit(0)
        # service -- the property that makes the logout loop unbounded
        # on purpose (login-manager's window-failure exit(0) loop).
        self.assertIn("// Clean exit (status == 0)", self.init)
        self.assertIn("if status == 0 {", self.init_code)
        self.assertIn("svc.crashes = 0;", self.init_code)
        reset = self.init_code.index("if status == 0 {")
        increment = self.init_code.index("svc.crashes += 1;")
        self.assertLess(
            reset, increment,
            "crash reset (status == 0) must come before the increment",
        )

    def test_crash_count_is_respawn_guarded_and_strictly_gt(self):
        # Counting happens only for respawnable services, and give-up
        # fires only when crashes STRICTLY exceeds MAX_RESPAWNS (the
        # counter holds 1..=5 while respawning; the 6th event gives up).
        self.assertIn("if svc.respawn {", self.init_code)
        self.assertIn("svc.crashes += 1;", self.init_code)
        self.assertIn("if svc.crashes > MAX_RESPAWNS {", self.init_code)
        self.assertIn("svc.respawn = false;", self.init_code)
        respawn = self.init_code.index("if svc.respawn {")
        increment = self.init_code.index("svc.crashes += 1;")
        give_up = self.init_code.index("if svc.crashes > MAX_RESPAWNS {")
        self.assertLess(respawn, increment)
        self.assertLess(increment, give_up)

    def test_give_up_disables_respawn_so_no_further_counting(self):
        # Structural pins for "give-up stops the loop": exactly ONE
        # increment site exists in init (there is no second counter to
        # advance after respawn is disabled), and `svc.respawn = false;`
        # sits INSIDE the strictly-greater branch, not in the else
        # (respawn) branch. (Behaviorally: init clears svc.pid on every
        # exit, so a post-give-up exit is orphaned, never recounted.)
        code = self.init_code
        self.assertEqual(
            code.count("svc.crashes += 1;"), 1,
            "init must have exactly one crash-count site",
        )
        give_up = code.index("svc.respawn = false;")
        branch_open = code.index("if svc.crashes > MAX_RESPAWNS {")
        self.assertGreater(give_up, branch_open)
        # respawn = false must not be inside the else/respawn branch: the
        # text between it and the give-up branch close must not reach the
        # else keyword that precedes the respawn call.
        between = code[branch_open:give_up]
        self.assertNotIn("} else {", between)

    # --- session side (ade/src/service/session.rs) ---
    def test_exit_logout_is_zero_matching_init_reset(self):
        # The session-side of the contract: a deliberate logout is exit
        # code 0, which init's waitpid loop treats as the clean-exit reset
        # (test_zero_status_resets_crash_counter_first). A non-zero code
        # would accumulate toward MAX_RESPAWNS and eventually kill the
        # login service until reboot.
        self.assertIn("const EXIT_LOGOUT: i32 = 0;", self.session)
        # The doc comment must state init's side accurately -- it is the
        # cross-source agreement these pins enforce.
        self.assertIn("0 = clean logout", self.session)
        self.assertIn("init resets its crash counter and respawns the login service", self.session)
        self.assertIn("init counts it toward", self.session)
        self.assertIn("pub fn exit_code(&self) -> i32", self.session)
        # exit_code returns EXIT_LOGOUT (the const), not a hardcoded 0.
        fn = self.session_code.split("pub fn exit_code", 1)[1]
        self.assertIn("EXIT_LOGOUT", fn)

    # --- the wiring between them (ade/src/main.rs) ---
    def test_ade_passes_session_exit_code_to_init(self):
        # main.rs's end-of-session path: "[ade] session ended" then the
        # session's exit code -- exactly the value init's waitpid sees.
        self.assertIn('io::print_str("[ade] session ended\\n");', self.ade_main)
        self.assertIn("desktop.session.exit_code()", self.ade_main_code)
        # The comment on that line documents the contract in init terms.
        self.assertIn("init treats 0 as a clean exit and respawns", self.ade_main)

    def test_exit_code_const_matches_comment(self):
        # The const in session.rs and the comment in main.rs must agree on
        # 0: session.rs writes `const EXIT_LOGOUT: i32 = 0;` (colon before
        # the type), and main.rs's end-of-session comment spells out the
        # same value in init terms.
        self.assertIn("const EXIT_LOGOUT: i32 = 0;", self.session)
        self.assertIn("EXIT_LOGOUT = 0; init treats 0 as a clean exit", self.ade_main)


if __name__ == "__main__":
    unittest.main()
