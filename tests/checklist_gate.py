#!/usr/bin/env python3
"""CHECKLIST-OK gate: derives each §6 queue row's landed state from the gate
docs' CHECKLIST-OK markers + landing evidence, and asserts the queue's
'landed' column agrees — so the rewrite ticks items by adding an
evidence-quoting CHECKLIST-OK marker to the gate doc (the doc it must update
anyway when landing), never by hand-editing the queue's status cells.

Run modes:
  python3 tests/checklist_gate.py            # unittest (CI): consistency gate
  python3 tests/checklist_gate.py --report   # print the CHECKLIST summary
"""
import io
import os
import re
import sys
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ADE_DOCS = os.path.join(REPO_ROOT, "ade", "docs")
SL_DOC = os.path.join(ADE_DOCS, "session-lifecycle.md")

MARKER = "**CHECKLIST-OK**"
# Canonical marker: the bare bold token. A colon-inside-bold variant
# ("**CHECKLIST-OK:**", the "**Queue:**" markdown habit) is accepted too, so
# the rewrite's wording can't silently miss the gate.
_MARKER_COLON = "**CHECKLIST-OK:**"


def _has_marker(text):
    return MARKER in text or _MARKER_COLON in text

# K-id -> the landing-condition PASS/ok evidence the row's gate doc must
# carry once landed. Kept in lockstep with the queue table by
# test_evidence_matches_landing_column (the table is the source of truth).
LANDING_EVIDENCE = {
    "K1": "`GUI + device-manager reachability gate: PASS`",
    "K1-alt": "`giving up on .*login-manager`",
    "K2": "`[KBD] IRQ1 fired!`",
    "K3": "`ok N - gui::option1_*`",
    "K3-alt": "`ok N - gui::option2_*`",
    "K4": "`qemu_shell_test.exp`",
    "K5": "`ok N - gui::drmctl_set_mode_ok`",
    "K6": "`[init] service svc exited`",
    "K7": "`ok N - buddy::low_water_monotonic`",
    "K8": "`ok N - syscalls::clipboard_copy_roundtrip`",
    "K9": "`ok N - vfs::mknodat_creates_dev_node`",
}

# K1-alt's queue cell says "same doc" (no filename link) — its gate doc is
# the fix doc's K1 banner. Resolved explicitly when the row has no link.
_SAME_DOC = {"K1-alt": "kernel-gui-window-fix.md"}


def _read(p):
    with io.open(p, encoding="utf-8") as fh:
        return fh.read()


def _rows(text):
    """Parse the queue rows: | # | Change (doc) | What lands | Landing
    condition | landed |. Cells can contain literal pipes inside code spans
    (K2's col 1 has ``byte | (mods << 8)``), so the K-id is the first cell,
    'landed' is the LAST cell, the landing condition is the cell before it,
    and col 1 is the middle cells rejoined with '|'."""
    rows = {}
    for line in text.splitlines():
        if not line.startswith("| K"):
            continue
        parts = [p.strip() for p in line.split("|")]
        if len(parts) >= 7 and parts[1]:
            rows[parts[1]] = {
                "col1": "|".join(parts[2:-4]),
                "landing": parts[-3],
                "landed": parts[-2],
            }
    return rows


def _linked_doc(col1, kid):
    m = re.search(r"\]\(([^)]+\.md)\)", col1)
    if m:
        return m.group(1)
    return _SAME_DOC.get(kid)


def derive():
    """Return {kid: True/False} — landed iff the gate doc carries the
    CHECKLIST-OK marker AND the landing evidence token."""
    sl = _read(SL_DOC)
    rows = _rows(sl)
    docs = {}
    result = {}
    for kid, row in rows.items():
        doc = _linked_doc(row["col1"], kid)
        if doc is None:
            result[kid] = False
            continue
        if doc not in docs:
            p = os.path.join(ADE_DOCS, doc)
            docs[doc] = _read(p) if os.path.isfile(p) else ""
        text = docs[doc]
        tok = LANDING_EVIDENCE.get(kid)
        result[kid] = _has_marker(text) and (tok is None or tok in text)
    return result


def _report(derived):
    lines = []
    for kid in sorted(derived):
        state = "CHECKLIST-OK" if derived[kid] else "pending"
        lines.append("K-id %s: %s" % (kid, state))
    return "CHECKLIST: " + " | ".join(lines)


class TestChecklistGate(unittest.TestCase):
    maxDiff = None

    def test_rows_parse_with_landed_column(self):
        sl = _read(SL_DOC)
        self.assertIn("## 6. Kernel change queue", sl)
        rows = _rows(sl)
        for kid in LANDING_EVIDENCE:
            self.assertIn(kid, rows, "queue lost the %s row" % kid)
        for kid, row in rows.items():
            self.assertIn(row["landed"], ("pending", "CHECKLIST-OK"),
                          "%s row's landed cell is not a valid state" % kid)
            self.assertIsNotNone(_linked_doc(row["col1"], kid),
                                 "%s row has no resolvable gate doc" % kid)

    def test_evidence_matches_landing_column(self):
        # The script's evidence tokens must appear verbatim in the queue
        # table's Landing condition column — the table is the source of
        # truth, so a token drift trips here first.
        sl = _read(SL_DOC)
        rows = _rows(sl)
        for kid, tok in LANDING_EVIDENCE.items():
            self.assertIn(tok, rows[kid]["landing"],
                          "%s evidence token drifted from the queue's "
                          "Landing condition column" % kid)

    def test_landed_column_matches_doc_evidence(self):
        # The derived state (marker + evidence in the gate doc) must equal
        # the table's landed cell, in BOTH directions: no CHECKLIST-OK
        # without evidence, no evidence without CHECKLIST-OK.
        sl = _read(SL_DOC)
        rows = _rows(sl)
        derived = derive()
        for kid, row in rows.items():
            expected = "CHECKLIST-OK" if derived.get(kid) else "pending"
            self.assertEqual(row["landed"], expected,
                             "%s landed cell (%s) does not match the doc "
                             "evidence (%s)" % (kid, row["landed"], expected))

    def test_marker_convention_documented(self):
        sl = _read(SL_DOC)
        self.assertIn(MARKER, sl,
                      "session-lifecycle.md lost the CHECKLIST-OK marker "
                      "convention paragraph")
        self.assertIn("checklist_gate.py", sl,
                      "session-lifecycle.md lost the checklist_gate.py "
                      "reference")


if __name__ == "__main__":
    if "--report" in sys.argv:
        print(_report(derive()))
        # The report is informational; consistency is the CI gate below.
        sys.exit(0)
    unittest.main()
