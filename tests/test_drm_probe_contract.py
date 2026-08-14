#!/usr/bin/env python3
"""Host-runnable pins for the DRM set_mode probe (K5-gated).

tests/qemu_drm_probe.exp proves the F4 fix on real hardware: boot, launch
skysettings from the console shell (initrd bin/skysettings), sendkey 'r'
to cycle the resolution, and assert the kernel prints 'DRM: set_mode'
(kernel-drmctl-fix.md Fix 1). Until the kernel rewrite lands K5 the EINVAL
path prints nothing, so the harness defers via KERNEL-GATED (exit 0), the
same pattern as qemu_giveup_boot.exp. These tests pin the harness legs,
the sargasettings enabling hooks, the initrd entry, the CI wiring, and the
K5 queue-row reference so the machinery cannot silently drift.
"""
import io
import os
import unittest

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BS = chr(92)  # backslash, for escape-containing needles


def _read(rel):
    with io.open(os.path.join(REPO, rel), encoding="utf-8") as fh:
        return fh.read()


class TestDrmProbeContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.exp = _read(os.path.join("tests", "qemu_drm_probe.exp"))
        cls.sarga = _read(os.path.join("sargasettings", "src", "main.rs"))
        cls.initrd = _read("build_initrd.py")
        cls.ci = _read(os.path.join(".github", "workflows", "ci.yml"))
        cls.sl = _read(os.path.join("ade", "docs", "session-lifecycle.md"))
        cls.fix = _read(os.path.join("ade", "docs", "kernel-drmctl-fix.md"))

    def test_harness_legs_pinned(self):
        e = self.exp
        # Launch, ready marker, resolution key, routing tripmine, K5 assert.
        self.assertIn('send "skysettings\\r"', e)
        self.assertIn(BS + "[settings] ready", e)
        self.assertIn('sendkey_seq "r"', e)
        self.assertIn(BS + "[settings] applying", e)
        self.assertIn("DRM: set_mode", e)
        self.assertIn("KERNEL-GATED:", e)
        self.assertIn("PASS: DRM set_mode delivered", e)

    def test_sargasettings_enabling_hooks(self):
        # Raw source, NOT strip_rust: the pins include the very print
        # strings, which a comment/string stripper would delete.
        s = self.sarga
        self.assertIn('io::print_str("[settings] ready\\n")', s)
        self.assertIn('"[settings] applying {}x{}\\n"', s)
        self.assertIn("b'r' =>", s)
        self.assertIn("libsarga::gpu::set_mode(w, h, 32)", s)
        self.assertIn("settings.save()", s)

    def test_initrd_entry(self):
        # The settings binary must ship in the initrd under the app
        # catalog's exec path (/bin/skysettings).
        self.assertIn("'bin/skysettings':   'sargasettings',", self.initrd)

    def test_ci_step_wired_with_gate(self):
        c = self.ci
        self.assertIn("- name: DRM set_mode probe (K5-gated)", c)
        self.assertIn('expect tests/qemu_drm_probe.exp "$ISO" 240', c)
        # The KERNEL-GATED conditional must defer (exit 0), not fail.
        self.assertIn(
            'if grep -q "KERNEL-GATED:" qemu_drm_probe_log.txt; then', c)
        self.assertIn('exit 0', c)
        self.assertIn(
            'grep -q "PASS: DRM set_mode delivered" qemu_drm_probe_log.txt', c)
        # This file itself must be wired into the host-tests job.
        self.assertIn("python3 tests/test_drm_probe_contract.py", c)

    def test_k5_queue_row_references_probe(self):
        self.assertIn("tests/qemu_drm_probe.exp", self.sl)
        self.assertIn("`DRM: set_mode`", self.sl)
        self.assertIn("tests/qemu_drm_probe.exp", self.fix)


if __name__ == "__main__":
    unittest.main()
