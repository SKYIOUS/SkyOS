#!/usr/bin/env python3
"""Expect-style consume-semantics pattern matching over a growing log buffer.

The expect harnesses' core contract: when a pattern matches, the buffer is
discarded up to and including the match, so a LATER expect can only match
data that arrived after the previous match. A naive Python port that does
`if pattern in read_log()` against the whole accumulated log on every poll
never discards anything — so a marker that appeared once keeps satisfying
every later wait_for (a false positive).

probe_sendkey.py (the Phase B sendkey probe) hit exactly this: it waited for
"[login] window created" TWICE, and the second call passed only because the
first match was still in the buffer. ConsumeMatcher is the shared fix.

This module is also the SHARED SERIAL DRIVER: `poll_with_timeout` owns the
deadline / process-exit / sleep poll loop that probe_sendkey's wait_for,
run_ade_selftest_local's expect, and boot_stress's boot loop all used to
re-implement inline. The loop itself lives here once; the call sites pass
only their read/check/poll hooks.

Run:  python3 tests/test_probe_consume.py  (pins this contract)
"""

import time

# Outcomes of poll_with_timeout when no check hook fired.
EXITED = "EXITED"   # the poll hook reported the process ended
TIMEOUT = "TIMEOUT"  # the deadline passed without a hit


class ConsumeMatcher:
    """Matches patterns against a growing text buffer, consuming on hit.

    `search(text, pattern)` looks for `pattern` only at/after the consume
    point. On a hit it advances the consume point past the end of the
    match, so the same occurrence can never match again — exactly what
    expect's buffer discard does. Pass the full buffer on every call; the
    matcher tracks where it has read to.

    `pattern` is a plain string (matched with str.find) or a compiled
    regex (matched against the unconsumed tail; returns the matched text).
    """

    def __init__(self):
        self._consumed = 0

    def search(self, text, pattern):
        """True (or matched text, for regex) if `pattern` occurs at/after the
        consume point; consumes through the end of the match."""
        # Clamp defensively: if the buffer was ever truncated/recreated
        # mid-run (log rotation, harness restart), a stale consume point past
        # the new end would make find() return -1 forever. Cheap insurance
        # for a shared primitive.
        self._consumed = min(self._consumed, len(text))
        if isinstance(pattern, str):
            idx = text.find(pattern, self._consumed)
            if idx == -1:
                return False
            self._consumed = idx + len(pattern)
            return True
        m = pattern.search(text[self._consumed:])
        if not m:
            return None
        self._consumed += m.end()
        return m.group(0)

    def poll_with_timeout(self, check, *, timeout, read=None, poll=None,
                          sleep=0.5):
        """Shared serial-driver poll loop: the deadline/exit/sleep skeleton
        every QEMU-facing harness re-implemented inline.

        Each round: calls `read()` (if given) to refresh the buffer, then
        `check(text)`; returns `check`'s truthy result as-is (so callers
        distinguish WHICH pattern hit). If the `poll()` hook reports the
        process ended, returns EXITED. If the `timeout` (seconds, monotonic)
        passes without a hit, returns TIMEOUT. Sleeps `sleep` seconds
        between rounds.
        """
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            text = read() if read is not None else None
            result = check(text)
            if result:
                return result
            if poll is not None and poll():
                return EXITED
            time.sleep(sleep)
        return TIMEOUT

    def consumed(self):
        """Offset into the buffer that has been matched and discarded."""
        return self._consumed

    def reset(self):
        """Start over (new boot / new log file)."""
        self._consumed = 0
