#!/usr/bin/env python3
"""Host-runnable contract test pinning session-lifecycle.md's §6 Kernel
change queue — the rewrite's one landing checklist — so the checklist
itself cannot silently drift while the kernel rewrite is in flight:

1. The queue table lists the seven core rows (K1, K1-alt, K2, K3, K4,
   K8, K9), each carrying its gate-doc identity (kernel-gui-window-fix.md,
   the K1-alt Option 2 + 2b variant, kernel-keyboard-gate.md, kernel-gui-
   selftest-spec.md, kernel-tcsets-echo.md, kernel-owns-facility-audit.md).
2. Each row's harness-condition tokens (the 3rd column — the QEMU / selftest
   markers the row's gate asserts) stay exact.
3. Both gate docs carry their K-number banners back to §6
   (kernel-gui-window-fix.md: **K1 / K1-alt**; kernel-keyboard-gate.md:
   **K2**).
"""
import io
import os
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ADE_DOCS = os.path.join(REPO_ROOT, "ade", "docs")
SL_DOC = os.path.join(ADE_DOCS, "session-lifecycle.md")
FIX_DOC = os.path.join(ADE_DOCS, "kernel-gui-window-fix.md")
AUDIT_DOC = os.path.join(ADE_DOCS, "kernel-owns-facility-audit.md")
KBD_DOC = os.path.join(ADE_DOCS, "kernel-keyboard-gate.md")

# K-id -> (col-1 identity token, [col-3 harness-condition tokens]).
# The identity token anchors the row to its gate doc; K1-alt deliberately
# uses its Option 2 + 2b marker because its col 1 says "same doc" (the
# K1/K1-alt banner in the fix doc carries the filename link instead).
CORE_ROWS = {
    "K1": ("`kernel-gui-window-fix.md`",
           ["`qemu_gui_gate.exp`",
            "`GUI + device-manager reachability gate: PASS`",
            "`gui::option1_*`"]),
    "K1-alt": ("**Option 2 + 2b**",
               ["`giving up on .*login-manager`",
                "POSITIVE",
                "mutually exclusive"]),
    "K2": ("`kernel-keyboard-gate.md`",
           ["`[KBD] IRQ1 fired!`",
            "`sendkey tab`",
            "`[ade] session ended`",
            "`test_keymap`",
            "`test_session_end_gate`"]),
    "K3": ("`kernel-gui-selftest-spec.md`",
           ["`ok N - gui::option1_*`",
            "`not ok`"]),
    "K4": ("`kernel-tcsets-echo.md`",
           ["`tests/test_login_echo.py`",
            "`qemu_shell_test.exp`"]),
    "K5": ("`kernel-drmctl-fix.md`",
           ["`DRM: set_mode`",
            "`tests/qemu_drm_probe.exp`",
            "`ok N - gui::drmctl_set_mode_ok`",
            "`gui::drmctl_map_dumb_roundtrip`"]),
    "K8": ("`kernel-owns-facility-audit.md`",
           ["`ok N - syscalls::clipboard_copy_roundtrip`",
            "`tests/test_clipboard_contract.py`",
            "`tests/qemu_clipboard_probe.exp`",
            "`user_access`"]),
    "K9": ("`kernel-owns-facility-audit.md`",
           ["`ok N - vfs::mknodat_creates_dev_node`",
            "`tests/test_vahid_contract.py`",
            "`qemu_gui_gate.exp`"]),
}


def _read(p):
    with io.open(p, encoding="utf-8") as fh:
        return fh.read()


def _rows(text):
    """Parse the queue table rows: '| K-id | col1 | col2 | col3 |'.

    Cells can contain literal pipes inside code spans (K2's col 1 has
    ``byte | (mods << 8)``), so the cells are not a fixed split: the K-id
    is the first cell, col3 is the LAST cell, and col1 is the middle cells
    rejoined with '|'.
    """
    rows = {}
    for line in text.splitlines():
        if not line.startswith("| K"):
            continue
        parts = [p.strip() for p in line.split("|")]
        if len(parts) >= 7 and parts[1]:
            rows[parts[1]] = {
                "col1": "|".join(parts[2:-4]),
                "col3": parts[-3],
            }
    return rows


class TestKernelQueueContract(unittest.TestCase):
    maxDiff = None

    def test_queue_rows_present_and_linked(self):
        sl = _read(SL_DOC)
        # The §6 header itself is pinned so the gate-doc '§6' banners cannot
        # silently orphan the section.
        self.assertIn("## 6. Kernel change queue", sl,
                      "session-lifecycle.md lost the §6 queue section header")
        rows = _rows(sl)
        for kid, (identity, _tokens) in CORE_ROWS.items():
            self.assertIn(kid, rows,
                          "session-lifecycle.md §6 lost the %s row" % kid)
            self.assertIn(identity, rows[kid]["col1"],
                          "%s row no longer carries its gate-doc identity "
                          "%s" % (kid, identity))

    def test_harness_conditions_exact(self):
        sl = _read(SL_DOC)
        rows = _rows(sl)
        for kid, (_identity, tokens) in CORE_ROWS.items():
            row = rows.get(kid)
            self.assertIsNotNone(row, "%s row missing" % kid)
            for tok in tokens:
                self.assertIn(tok, row["col3"],
                              "%s row lost harness condition %s" % (kid, tok))

    def test_gate_doc_banners_pinned(self):
        fix = _read(FIX_DOC)
        self.assertIn("**K1 / K1-alt**", fix,
                      "kernel-gui-window-fix.md lost its K1/K1-alt banner")
        self.assertIn("`session-lifecycle.md`", fix)
        self.assertIn("§6", fix)
        kbd = _read(KBD_DOC)
        self.assertIn("**K2**", kbd,
                      "kernel-keyboard-gate.md lost its K2 banner")
        self.assertIn("`session-lifecycle.md`", kbd)
        self.assertIn("§6", kbd)
        audit = _read(AUDIT_DOC)
        self.assertIn("**K8 / K9**", audit,
                      "kernel-owns-facility-audit.md lost its K8/K9 "
                      "banner")
        self.assertIn("`session-lifecycle.md`", audit)
        self.assertIn("§6", audit)


if __name__ == "__main__":
    unittest.main()
