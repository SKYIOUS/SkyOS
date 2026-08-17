#!/usr/bin/env python3
"""Runtime probe (Phase B): does QEMU monitor sendkey reach login-manager?

Boots the given ISO with -monitor stdio (no expect needed), drives the GUI
login via sendkey (root / tab / skyos / ret), and reports whether the login
reaches "[ade] session established". Evidence for the source-level probe in
qemu_gui_login.exp: IRQ1 -> GUI_SCANCODE_QUEUE -> gui_refresh_task ->
COMPOSITOR.handle_keyboard -> window.key_events -> SYS_GUI_GET_KEY.

Matching uses expect-style consume semantics (tests/expect_consume.py): each
wait_for discards everything through the end of its match, so a marker that
appeared once can never satisfy a later wait_for. The original port
scanned the whole accumulated log every poll and never discarded matched
content, which false-positived when the same marker was waited for twice
off a single occurrence.

Usage: python3 tests/probe_sendkey.py <iso_path> [ovmf_path]
"""
import os
import subprocess
import sys
import time

from expect_consume import ConsumeMatcher, EXITED, TIMEOUT

ISO = sys.argv[1]
OVMF = sys.argv[2] if len(sys.argv) > 2 else os.path.abspath("kernel/OVMF.fd")
LOG = "probe_serial.log"

MATCHER = ConsumeMatcher()

if os.path.exists(LOG):
    os.remove(LOG)

qemu = subprocess.Popen(
    [
        "qemu-system-x86_64",
        "-bios", OVMF,
        "-cdrom", ISO,
        "-m", "512M", "-smp", "2",
        "-nographic", "-no-reboot",
        "-serial", "file:" + LOG,
        "-monitor", "stdio",
    ],
    stdin=subprocess.PIPE,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
    text=True,
)


def send(cmd: str) -> None:
    qemu.stdin.write(cmd + "\r")
    qemu.stdin.flush()


def read_log() -> str:
    try:
        with open(LOG, encoding="utf-8", errors="replace") as fh:
            return fh.read()
    except FileNotFoundError:
        return ""


def wait_for(pattern: str, timeout: float, what: str) -> bool:
    """Wait for `pattern` in NEW log output (expect-style consume semantics).

    MATCHER.search reads only data at/after the consume point and discards
    through the end of a match, so each call waits for a FRESH occurrence.
    This is the behavior the old whole-buffer scan broke: a marker
    that appeared once kept satisfying every later wait.

    The deadline/exit/sleep loop is the shared serial driver
    (ConsumeMatcher.poll_with_timeout), not a local copy.
    """
    result = MATCHER.poll_with_timeout(
        lambda text: MATCHER.search(text, pattern),
        timeout=timeout,
        read=read_log,
        poll=lambda: qemu.poll() is not None,
    )
    if result is True:
        print(f"PASS: {what}")
        return True
    if result == EXITED:
        print(f"FAIL: QEMU exited while waiting for {what}")
        return False
    print(f"FAIL: timeout waiting for {what} (pattern {pattern!r})")
    return False


try:
    # Match the stable prefix only — the repo's qemu_gui_gate.exp documents
    # that the kernel's serial path garbles service names (e.g. "[TTY0W]
    # len=5"), so a full-name match can hang.
    if not wait_for("[init] starting service:", 90,
                    "init reached its service-spawn loop"):
        print("note: no service spawn; tail of log:")
        print(read_log()[-3000:])
        sys.exit(1)
    if not wait_for("[login] window created", 30,
                    "login-manager created GUI window"):
        print("note: window-marker missing; tail of log:")
        print(read_log()[-3000:])
        sys.exit(1)

    print("=== injecting root / skyos via monitor sendkey ===")
    send("sendkey r o o t")
    time.sleep(1)
    send("sendkey tab")
    time.sleep(1)
    send("sendkey s k y o s")
    time.sleep(1)
    send("sendkey ret")

    if wait_for("[ade] session established", 30,
                "GUI login reached the ade session"):
        print("\n=== PROBE: sendkey routing CONFIRMED ===")
        sys.exit(0)

    print("=== PROBE: login did not complete; log tail ===")
    print(read_log()[-3000:])
    sys.exit(1)
finally:
    qemu.kill()
    qemu.wait(timeout=5)
