#!/usr/bin/env python3
"""Host-runnable unit tests for login's getty echo contract (no QEMU).

Pins `login/src/main.rs::echo_off` / `echo_on` / `read_password` — the
getty's password-hiding logic:

  echo_off(fd)   -> TCGETS the current termios, save c_lflag, clear ECHO
                    (0x8) in c_lflag, TCSETS it back; returns the saved
                    lflag on success, None on ANY ioctl failure (so the
                    caller skips the restore and a bogus 0 can never
                    clobber real termios once the kernel implements TCSETS).
  echo_on(fd, l) -> TCGETS, set c_lflag = l, TCSETS; silently no-ops if
                    TCGETS fails.
  read_password  -> echo_off, read one line, echo_on(restore) ONLY if
                    echo_off returned Some.

The kernel's TCSETS is currently a no-op returning 0 (syscalls/mod.rs),
so this cannot be exercised in QEMU today — the host test is the only
execution of the contract. Two layers of pinning, same as
test_login_flow.py:

  1. A faithful Python port driven through an injectable fake ioctl
     channel, so TCGETS-failure / TCSETS-failure / bit-clear / field-
     preservation semantics are executed on the host.
  2. Source-contract pins that grep login/src/main.rs and libsarga's
     ioctls module, so a drift in the Rust (const value, mask expression,
     restore-skip) fails CI before any boot.

Run:  python3 tests/test_login_echo.py
"""

import os
import re
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LOGIN_RS = os.path.join(REPO_ROOT, "login", "src", "main.rs")
LIBSARGA_IO_RS = os.path.join(REPO_ROOT, "libsarga", "src", "io.rs")

# ioctl request numbers (mirrors libsarga/src/io.rs ioctls module).
TCGETS = 0x5401
TCSETS = 0x5402

# ECHO bit in termios c_lflag (POSIX; pinned to login/src/main.rs).
ECHO = 0x8

CC_SIZE = 19


def _termios(**kw):
    """Zeroed Termios mirror (repr(C): 4 u32 + c_cc [u8; 19])."""
    t = {
        "c_iflag": 0,
        "c_oflag": 0,
        "c_cflag": 0,
        "c_lflag": 0,
        "c_cc": [0] * CC_SIZE,
    }
    t.update(kw)
    return t


class FakeTty:
    """In-memory tty device + ioctl channel.

    Behaves like the kernel's sys_ioctl: TCGETS copies the device state
    into the caller's buffer; TCSETS copies the caller's buffer into the
    device. `fail` is a set of request numbers that return Err(None).
    Records every request for call-order assertions.
    """

    def __init__(self, **state):
        self.dev = _termios(**state)
        self.fail = set()
        self.calls = []

    def ioctl(self, _fd, request, argp):
        self.calls.append(request)
        if request in self.fail:
            return None  # Err
        if request == TCGETS:
            argp.update(self.dev)
        elif request == TCSETS:
            self.dev.update(argp)
        return 0


# --- Faithful ports of login/src/main.rs --------------------------------
def echo_off(fd, ioctl):
    """Port of login::echo_off -> Option<u32>."""
    t = _termios()
    if ioctl(fd, TCGETS, t) is None:
        return None
    saved = t["c_lflag"]
    t["c_lflag"] &= ~ECHO
    if ioctl(fd, TCSETS, t) is None:
        return None
    return saved


def echo_on(fd, lflag, ioctl):
    """Port of login::echo_on -> () (silent no-op on TCGETS failure)."""
    t = _termios()
    if ioctl(fd, TCGETS, t) is None:
        return
    t["c_lflag"] = lflag
    ioctl(fd, TCSETS, t)


def read_password(fd, ioctl, read_line):
    """Port of login::read_password -> Result<Option<Vec<u8>>, Error>.

    The restore is skipped when echo_off failed (non-tty fd), so a bogus
    lflag can never clobber real termios.
    """
    saved = echo_off(fd, ioctl)
    r = read_line(fd)
    if saved is not None:
        echo_on(fd, saved, ioctl)
    return r


class TestEchoOff(unittest.TestCase):
    def test_tcgets_failure_returns_none_no_tcsets(self):
        tty = FakeTty(c_lflag=0xB)
        tty.fail = {TCGETS}
        self.assertIsNone(echo_off(0, tty.ioctl))
        self.assertEqual(tty.calls, [TCGETS])  # no TCSETS attempted
        self.assertEqual(tty.dev["c_lflag"], 0xB)  # device untouched

    def test_tcsets_failure_returns_none(self):
        tty = FakeTty(c_lflag=0xB)
        tty.fail = {TCSETS}
        self.assertIsNone(echo_off(0, tty.ioctl))
        self.assertEqual(tty.calls, [TCGETS, TCSETS])
        self.assertEqual(tty.dev["c_lflag"], 0xB)  # store rejected

    def test_clears_echo_bit_and_returns_saved(self):
        tty = FakeTty(c_lflag=0xB)  # ISIG|ICANON|ECHO
        self.assertEqual(echo_off(0, tty.ioctl), 0xB)  # saved lflag
        self.assertEqual(tty.dev["c_lflag"], 0xB & ~ECHO)  # ECHO cleared
        self.assertEqual(tty.dev["c_lflag"] & ECHO, 0)  # ECHO bit cleared

    def test_other_fields_preserved(self):
        tty = FakeTty(
            c_iflag=0x100,
            c_oflag=0x2,
            c_cflag=0xBF,
            c_lflag=0xB,
            c_cc=[7] * CC_SIZE,
        )
        echo_off(0, tty.ioctl)
        self.assertEqual(tty.dev["c_iflag"], 0x100)
        self.assertEqual(tty.dev["c_oflag"], 0x2)
        self.assertEqual(tty.dev["c_cflag"], 0xBF)
        self.assertEqual(tty.dev["c_cc"], [7] * CC_SIZE)
        self.assertEqual(tty.dev["c_lflag"], 0xB & ~ECHO)

    def test_no_echo_bit_noop(self):
        # lflag without ECHO (e.g. kernel's advertised 0x5): clear is a no-op.
        tty = FakeTty(c_lflag=0x5)
        self.assertEqual(echo_off(0, tty.ioctl), 0x5)
        self.assertEqual(tty.dev["c_lflag"], 0x5)


class TestEchoOn(unittest.TestCase):
    def test_restores_saved_lflag_preserves_fields(self):
        tty = FakeTty(c_iflag=0x100, c_oflag=0x2, c_cflag=0xBF, c_lflag=0xB)
        saved = echo_off(0, tty.ioctl)
        echo_on(0, saved, tty.ioctl)
        self.assertEqual(tty.dev["c_lflag"], 0xB)  # fully restored
        self.assertEqual(tty.dev["c_iflag"], 0x100)
        self.assertEqual(tty.dev["c_oflag"], 0x2)
        self.assertEqual(tty.dev["c_cflag"], 0xBF)

    def test_tcgets_failure_silent_no_tcsets(self):
        tty = FakeTty(c_lflag=0xB)
        tty.fail = {TCGETS}
        echo_on(0, 0xB, tty.ioctl)  # must not panic / must not store
        self.assertEqual(tty.calls, [TCGETS])
        self.assertEqual(tty.dev["c_lflag"], 0xB)


class TestReadPassword(unittest.TestCase):
    def test_echo_off_during_read_restored_after(self):
        tty = FakeTty(c_lflag=0xB)
        seen_during_read = []

        def read_line(_fd):
            # Snapshot the device lflag while the password is being read:
            # it must be ECHO-cleared at that exact moment.
            seen_during_read.append(tty.dev["c_lflag"])
            return [b"skyos"]

        r = read_password(0, tty.ioctl, read_line)
        self.assertEqual(r, [b"skyos"])
        self.assertEqual(seen_during_read, [0xB & ~ECHO])  # hidden during read
        self.assertEqual(tty.dev["c_lflag"], 0xB)  # restored after

    def test_restore_skipped_when_echo_off_failed(self):
        # Non-tty fd (TCGETS fails): echo_off returns None -> no restore.
        tty = FakeTty(c_lflag=0xB)
        tty.fail = {TCGETS}
        r = read_password(0, tty.ioctl, lambda _fd: [b"x"])
        self.assertEqual(r, [b"x"])
        self.assertEqual(tty.calls, [TCGETS])  # echo_on never ran
        self.assertEqual(tty.dev["c_lflag"], 0xB)

    def test_restore_skipped_when_tcsets_fails(self):
        # TCSETS fails after a successful TCGETS: echo_off still returns
        # None, so read_password must skip echo_on -- the "bogus 0 can
        # never clobber real termios" clause of the Rust comment. The
        # device must be left exactly as it was (all-or-nothing syscall).
        tty = FakeTty(c_lflag=0xB)
        tty.fail = {TCSETS}
        r = read_password(0, tty.ioctl, lambda _fd: [b"x"])
        self.assertEqual(r, [b"x"])
        self.assertEqual(tty.calls, [TCGETS, TCSETS])  # echo_on never ran
        self.assertEqual(tty.dev["c_lflag"], 0xB)  # untouched, no clobber


class TestSourceContract(unittest.TestCase):
    """Grep-pins so a Rust drift fails CI before any boot."""

    @classmethod
    def setUpClass(cls):
        with open(LOGIN_RS, encoding="utf-8") as fh:
            cls.login = fh.read()
        with open(LIBSARGA_IO_RS, encoding="utf-8") as fh:
            cls.io = fh.read()

    def test_echo_const_is_0x8(self):
        self.assertIn("const ECHO: u32 = 0x8;", self.login)

    def test_clear_is_bitwise_and_not(self):
        self.assertIn("t.c_lflag &= !ECHO;", self.login)
        self.assertIn("let saved = t.c_lflag;", self.login)
        self.assertIn("Some(saved)", self.login)

    def test_restore_skipped_on_none(self):
        self.assertIn("if let Some(lflag) = saved {", self.login)
        self.assertIn("echo_on(fd, lflag);", self.login)

    def test_echo_on_sets_exact_lflag(self):
        self.assertIn("t.c_lflag = lflag;", self.login)

    def test_ioctls_match_libsarga(self):
        # login must call through libsarga's ioctls module.
        self.assertIn("ioctls::TCGETS", self.login)
        self.assertIn("ioctls::TCSETS", self.login)
        # libsarga must keep the Linux termios numbers.
        self.assertIn("pub const TCGETS: u64 = 0x5401;", self.io)
        self.assertIn("pub const TCSETS: u64 = 0x5402;", self.io)

    def test_termios_layout_mirror(self):
        # repr(C), 4 u32 + c_cc [u8; 19] -- the kernel ABI both sides share.
        # Order is pinned, not just presence: reordering fields (e.g. c_cc
        # before c_lflag) would silently break the shared repr(C) layout
        # while all presence checks still pass.
        m = re.search(
            r"#\[repr\(C\)\]\nstruct Termios \{\n"
            r"\s+c_iflag: u32,\n"
            r"\s+c_oflag: u32,\n"
            r"\s+c_cflag: u32,\n"
            r"\s+c_lflag: u32,\n"
            r"\s+c_cc: \[u8; 19\],\n\}",
            self.login,
        )
        self.assertIsNotNone(m, "Termios repr(C) field order/layout drifted")


if __name__ == "__main__":
    unittest.main()
