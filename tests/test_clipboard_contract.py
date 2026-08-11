#!/usr/bin/env python3
"""Host-runnable source-contract tests for the F1 clipboard split (no QEMU).

The clipboard had two disconnected stores:

  BEFORE F1 (the bug):  sash yank  -> kernel clipboard  (SYS_CLIPBOARD=125)
                        ade portal -> userspace ClipboardManager only
  A yank on the console and a copy in a GUI app landed in DIFFERENT buffers,
  so cross-system paste was broken by construction.

  AFTER F1 (Aug 10, 2026):  sash yank  -> kernel clipboard  (unchanged)
                            ade portal copy  -> kernel write + manager history
                            ade portal paste -> kernel read
  The kernel store is the single shared buffer; ClipboardManager survives
  only as the history overlay that feeds the clipboard panel
  (render/overlay.rs draw_clipboard) and the direct selftest.

These tests pin the AFTER state as the tripwire: if a future change reverts
the portal to reading the manager for paste (the BEFORE state), or sash stops
flushing yanks to the kernel, or the wrapper guards get deleted as "dead
code", this suite fails before any QEMU boot runs.

Run:  python3 tests/test_clipboard_contract.py
"""
import os
import re
import sys
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scan_rust import strip_rust  # noqa: E402

SASH_RS = os.path.join(REPO_ROOT, "sash", "src", "readline.rs")
DESKTOP_API_RS = os.path.join(REPO_ROOT, "ade", "src", "util", "desktop_api", "clipboard.rs")
PORTAL_RS = os.path.join(REPO_ROOT, "ade", "src", "sec", "portal", "clipboard.rs")
MANAGER_RS = os.path.join(REPO_ROOT, "ade", "src", "service", "clipboard.rs")
OVERLAY_RS = os.path.join(REPO_ROOT, "ade", "src", "render", "overlay.rs")
LIBSARGA_IO_RS = os.path.join(REPO_ROOT, "libsarga", "src", "io.rs")


def _read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


class TestSashYankWritesKernel(unittest.TestCase):
    """Every yank in sash's line editor must flush to the kernel store."""

    def test_yank_calls_clipboard_write(self):
        code = strip_rust(_read(SASH_RS))
        sites = code.count("clipboard_write(yank_buf.as_bytes())")
        self.assertGreaterEqual(
            sites, 5,
            "sash yank sites must all write the yank buffer to the kernel "
            "clipboard (SYS_CLIPBOARD=125); got %d sites" % sites,
        )

    def test_wrapper_write_targets_syscall_125(self):
        code = strip_rust(_read(LIBSARGA_IO_RS))
        start = code.index("pub fn clipboard_write")
        end = code.index("pub fn clipboard_len")
        region = code[start:end]
        self.assertIn(
            "syscall3(125, 1,", region,
            "clipboard_write must be the libsarga wrapper for SYS_CLIPBOARD "
            "mode 1 (write)",
        )


class TestAdePortalUsesKernelStore(unittest.TestCase):
    """The perm-gated portal path must route through the kernel store."""

    def test_copy_writes_kernel_before_history(self):
        code = strip_rust(_read(DESKTOP_API_RS))
        self.assertIn("libsarga::io::clipboard_write(", code)
        self.assertIn("desktop.services.clipboard.copy(", code)
        self.assertLess(
            code.index("libsarga::io::clipboard_write("),
            code.index("desktop.services.clipboard.copy("),
            "copy must write the kernel store BEFORE recording history",
        )

    def test_paste_reads_kernel_store(self):
        code = strip_rust(_read(DESKTOP_API_RS))
        self.assertIn("libsarga::io::clipboard_len()", code)
        self.assertIn("libsarga::io::clipboard_read(&mut buf)", code)
        self.assertNotIn(
            "clipboard.paste()", code,
            "portal paste must read the KERNEL store, not ClipboardManager "
            "(the BEFORE-F1 split); a revert trips this test",
        )
        self.assertNotIn(
            "syscall3(", code,
            "the desktop API must go through the libsarga wrappers, not raw "
            "syscalls",
        )

    def test_portal_never_consults_manager(self):
        code = strip_rust(_read(PORTAL_RS))
        self.assertNotIn("clipboard.paste()", code)
        self.assertNotIn("services.clipboard", code)

    def test_only_selftest_consumes_manager_paste(self):
        hits = []
        for root, _dirs, files in os.walk(os.path.join(REPO_ROOT, "ade", "src")):
            for name in files:
                if not name.endswith(".rs"):
                    continue
                path = os.path.join(root, name)
                code = strip_rust(_read(path))
                if "clipboard.paste()" in code:
                    hits.append(path)
        self.assertEqual(
            len(hits), 1,
            "ClipboardManager::paste must be consumed only by the direct "
            "selftest (history overlay role); found: %r" % hits,
        )
        self.assertTrue(
            hits and hits[0].endswith(os.path.join("util", "testing", "services.rs")),
            "the single manager.paste() consumer must be util/testing/services.rs",
        )


class TestManagerIsHistoryOverlay(unittest.TestCase):
    """ClipboardManager keeps its buffer/history API for the panel."""

    def test_manager_keeps_history_and_paste_api(self):
        code = strip_rust(_read(MANAGER_RS))
        self.assertIn("history: Vec<ClipboardEntry>", code)
        self.assertIn("pub fn paste(&self)", code)
        self.assertIn("pub fn history(&self)", code)
        self.assertIn("if self.history.len() > 16", code)

    def test_panel_reads_history(self):
        code = strip_rust(_read(OVERLAY_RS))
        self.assertIn("draw_clipboard", code)
        self.assertIn("cb.history()", code)


class TestWrapperGuardsKept(unittest.TestCase):
    """The 125 wrapper guards are live i64 errno detection, not dead code."""

    def test_read_and_len_guards_present(self):
        code = strip_rust(_read(LIBSARGA_IO_RS))
        read_start = code.index("pub fn clipboard_read")
        read_end = code.index("pub fn clipboard_write")
        len_start = code.index("pub fn clipboard_len")
        len_end = code.index("pub fn notify")
        self.assertIn(
            "if r < 0", code[read_start:read_end],
            "clipboard_read keeps its i64 errno guard (syscall1/3 return i64)",
        )
        self.assertIn(
            "if r < 0", code[len_start:len_end],
            "clipboard_len keeps its i64 errno guard (syscall1/3 return i64)",
        )


if __name__ == "__main__":
    unittest.main()
