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

Run:  python3 tests/test_probe_consume.py  (pins this contract)
"""


class ConsumeMatcher:
    """Matches patterns against a growing text buffer, consuming on hit.

    `search(text, pattern)` looks for `pattern` only at/after the consume
    point. On a hit it advances the consume point past the end of the
    match, so the same occurrence can never match again — exactly what
    expect's buffer discard does. Pass the full buffer on every call; the
    matcher tracks where it has read to.
    """

    def __init__(self):
        self._consumed = 0

    def search(self, text, pattern):
        """True if `pattern` occurs at/after the consume point (consumes it)."""
        # Clamp defensively: if the buffer was ever truncated/recreated
        # mid-run (log rotation, harness restart), a stale consume point past
        # the new end would make find() return -1 forever. Cheap insurance
        # for a shared primitive.
        self._consumed = min(self._consumed, len(text))
        idx = text.find(pattern, self._consumed)
        if idx == -1:
            return False
        self._consumed = idx + len(pattern)
        return True

    def consumed(self):
        """Offset into the buffer that has been matched and discarded."""
        return self._consumed

    def reset(self):
        """Start over (new boot / new log file)."""
        self._consumed = 0
