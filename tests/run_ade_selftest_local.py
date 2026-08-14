#!/usr/bin/env python3
"""Local expect-style re-run of tests/qemu_ade_selftest.exp (no expect.exe).

Boots the ISO under QEMU, drives the getty attempt-cap probe (10 wrong
passwords -> 30s backoff -> reset confirm), logs in as root/skyos, runs
`ade --selftest`, and asserts the PASS markers the ci.yml Verify step
greps -- including the widened FAIL alternation and the dedicated
logout/input-suite PASS greps. Exits 0 only when everything passed.

Use:  python3 run_ade_selftest_local.py <iso> [boot_timeout] [cpu]
      e.g. ... skyos-0.6.0.iso 240 "qemu64,-smep"
"""
import re
import subprocess
import sys

from expect_consume import ConsumeMatcher, EXITED, TIMEOUT

OVMF = "OVMF.fd"


class Harness:
    def __init__(self, iso, timeout=240, cpu=None):
        self.iso = iso
        self.timeout = timeout
        self.cpu = cpu
        self.m = ConsumeMatcher()
        self.proc = None
        self.buf = ""
        self.log = []

    def start(self):
        cmd = [
            "qemu-system-x86_64",
            "-bios", OVMF,
            "-cdrom", self.iso,
            "-m", "512M", "-smp", "2",
            "-nographic", "-no-reboot",
            "-serial", "mon:stdio",
            "-device", "e1000,netdev=net0",
            "-netdev", "user,id=net0",
        ]
        if self.cpu:
            cmd += ["-cpu", self.cpu]
        self.proc = subprocess.Popen(
            cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
        )

    def read(self):
        try:
            chunk = self.proc.stdout.read(1)
        except Exception:
            return
        if chunk:
            self.buf += chunk
            self.log.append(chunk)
            sys.stdout.write(chunk)
            sys.stdout.flush()

    def expect(self, patterns, timeout, what):
        """Wait for any of `patterns` (str or compiled regex). Returns the
        matched pattern, "EOF" (process ended), or "TIMEOUT"."""
        # The poll loop is the shared serial driver
        # (ConsumeMatcher.poll_with_timeout); only the hooks are local.
        def check(_text):
            for pat in patterns:
                hit = self.m.search(self.buf, pat)
                if hit:
                    return pat
            return None

        result = self.m.poll_with_timeout(
            check, timeout=timeout, read=self.read,
            poll=lambda: self.proc.poll() is not None, sleep=0.05)
        if result == EXITED:
            self.read()
            return "EOF"
        if result == TIMEOUT:
            return "TIMEOUT"
        return result

    def send(self, text):
        self.proc.stdin.write(text)
        self.proc.stdin.flush()

    def send_bad_password(self):
        self.send("root\r")
        r = self.expect(["Password:", "SARGA OS PANIC", "login: "], 20,
                        "password prompt")
        self.send("not-the-password\r")
        r = self.expect(["Login incorrect", "Too many failed attempts - pausing 30s",
                         "SARGA OS PANIC"], 20, "wrong-password rejection")
        return r

    def monitor_cmd(self, cmd):
        """Toggle to the QEMU monitor (Ctrl-A c), run a command, toggle back."""
        self.send("\x01c")
        r = self.expect(["(qemu) "], 5, "monitor prompt")
        if r != "(qemu) ":
            print("FAIL: QEMU monitor not reachable (%s)" % r)
            return False
        self.send(cmd + "\r")
        r = self.expect(["(qemu) "], 5, "monitor ack")
        if r != "(qemu) ":
            print("FAIL: monitor command '%s' not acknowledged (%s)" % (cmd, r))
            return False
        self.send("\x01c")
        return True

    def stop(self):
        if self.proc and self.proc.poll() is None:
            self.proc.terminate()


def main():
    iso = sys.argv[1]
    timeout = float(sys.argv[2]) if len(sys.argv) > 2 else 240
    cpu = sys.argv[3] if len(sys.argv) > 3 else None
    h = Harness(iso, timeout, cpu)
    h.start()
    try:
        # 1. Boot to the console login prompt.
        r = h.expect(["login: ", "SARGA OS PANIC", "Too many failed attempts"], timeout,
                     "boot to login prompt")
        if r != "login: ":
            print("FAIL: no login prompt (%s)" % r)
            return 1
        print("PASS: boot reached login prompt")

        # 1b. Attempt-cap probe: 10 wrong passwords -> cap -> 30s backoff
        #     -> reset confirm. (The exp asserts no '[init] starting service:
        #     getty' respawn throughout; we check the log after.)
        for i in range(1, 10):
            h.send_bad_password()
            r = h.expect(["login: ", "SARGA OS PANIC", "Too many failed attempts - pausing 30s"],
                         30, "re-prompt after attempt %d" % i)
            if r != "login: ":
                print("FAIL: no re-prompt after attempt %d (%s)" % (i, r))
                return 1
            print("PASS: attempt %d rejected, re-prompted" % i)

        # 10th attempt triggers the cap.
        h.send_bad_password()
        r = h.expect(["Too many failed attempts - pausing 30s", "login: ",
                      "SARGA OS PANIC"], 30, "attempt cap on 10th")
        if r != "Too many failed attempts - pausing 30s":
            print("FAIL: cap marker not found (%s)" % r)
            return 1
        print("PASS: attempt cap activated on 10th failure")

        # 30s backoff, then the getty re-prompts.
        r = h.expect(["login: ", "SARGA OS PANIC"], 60, "re-prompt after backoff")
        if r != "login: ":
            print("FAIL: no re-prompt after 30s backoff (%s)" % r)
            return 1
        print("PASS: getty re-prompted after 30s backoff")

        # 11th attempt: counter reset.
        h.send_bad_password()
        r = h.expect(["login: ", "SARGA OS PANIC"], 30, "post-cap re-prompt")
        if r != "login: ":
            print("FAIL: no re-prompt post-cap (%s)" % r)
            return 1
        print("PASS: post-cap attempt rejected (counter reset)")

        # No getty respawn anywhere: init's respawn accounting stayed quiet.
        if "[init] starting service: getty" in "".join(h.log):
            print("FAIL: getty respawned during the cap probe")
            return 1

        # 2. Correct login.
        h.send("root\r")
        r = h.expect(["Password:", "Login incorrect", "SARGA OS PANIC"], 20,
                     "password prompt")
        if r != "Password:":
            print("FAIL: real login rejected after cap probe (%s)" % r)
            return 1
        h.send("skyos\r")
        r = h.expect(["sash[", "Login incorrect", "SARGA OS PANIC"], 30, "shell")
        if r != "sash[":
            print("FAIL: shell prompt never appeared (%s)" % r)
            return 1
        print("PASS: logged in, shell ready")

        # 3. Run the selftest suite.
        h.send("ade --selftest\r")

        # The markers the ci.yml Verify step greps (widened FAIL
        # alternation + dedicated logout/input-suite PASS greps).
        required = [
            "PASS test_logout_protocol_from_chord",
            "PASS test_logout_inert_with_window_open",
            "PASS test_keymap",
            "PASS test_from_raw",
            "PASS test_session_end_gate",
        ]
        for marker in required:
            r = h.expect(["[test] " + marker, "[test] FAIL", "SARGA OS PANIC"], 90,
                         "marker " + marker)
            if r != "[test] " + marker:
                print("FAIL: %s never reported (%s)" % (marker, r))
                return 1
            print("PASS: %s reported" % marker)

        # 4. The aggregate verdict.
        r = h.expect(["selftest PASS", "selftest FAIL", "SARGA OS PANIC"], 60,
                     "selftest verdict")
        if r != "selftest PASS":
            print("FAIL: no 'selftest PASS' verdict (%s)" % r)
            return 1
        print("PASS: ade selftest suite passed")

        # 5. The widened FAIL alternation must NOT have fired: any
        #    '[test] FAIL test_(...)' line would have been caught above,
        #    but double-check the log for the exact alternation set.
        alt = r"\[test\] FAIL test_(a11y_|tooltip_|focus_|logout_protocol_from_chord|logout_inert_with_window_open|keymap|from_raw|session_end_gate)"
        if re.search(alt, "".join(h.log)):
            print("FAIL: a widened-FAIL-pattern line present in the log")
            return 1

        # 6. Real-hardware Esc session-end probe (byte path): the
        #    selftest verdict exited ade; launch it interactively, wait
        #    for the desktop, then inject a REAL hardware Esc via the
        #    QEMU monitor (sendkey esc -> IRQ1 -> 0x1B -> a11y Esc arm).
        #    On the empty desktop Esc must unwind the session with the
        #    rich marker - the byte-deliverable contract proven on real
        #    input, not synthetic events.
        r = h.expect(["sash[", "SARGA OS PANIC"], 20,
                     "shell after selftest verdict")
        if r != "sash[":
            print("FAIL: shell did not return after selftest verdict (%s)" % r)
            return 1
        print("PASS: shell returned after selftest verdict")
        h.send("ade\r")
        r = h.expect(["[ade] session established", "[ade] failed to create window",
                      "SARGA OS PANIC"], 30, "interactive ade desktop")
        if r != "[ade] session established":
            print("FAIL: ade desktop never established (%s)" % r)
            return 1
        print("PASS: interactive ade desktop established")
        if not h.monitor_cmd("sendkey esc"):
            return 1
        r = h.expect(["[ade] session ended code=0 ending=true", "SARGA OS PANIC"],
                     20, "hardware Esc session end")
        if r != "[ade] session ended code=0 ending=true":
            print("FAIL: hardware Esc did not end the session (%s)" % r)
            return 1
        print("PASS: hardware Esc ended the session - '[ade] session ended code=0 ending=true'")
        r = h.expect(["sash[", "SARGA OS PANIC"], 20,
                     "shell after session ended")
        if r != "sash[":
            print("FAIL: ade did not exit after session ended (%s)" % r)
            return 1
        print("PASS: ade exited cleanly, shell returned")

        print("\n=== ADE selftest integration: PASS ===")
        return 0
    finally:
        h.stop()


if __name__ == "__main__":
    sys.exit(main())
