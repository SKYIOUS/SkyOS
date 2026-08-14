#!/usr/bin/env python3
"""Host-runnable regression test for the ISO builders (no QEMU).

Pins the two pycdlib fixes in scripts/make_iso.py that made the fallback
ISO path actually produce a UEFI-bootable ISOHybrid:

1. `add_fp` positional-arg fix — pycdlib's signature is
   `add_fp(fp, length, iso_path=...)`, so the README file must be added
   with the byte length as the second positional argument (the old call
   passed the iso_path there, which pycdlib rejected).
2. El Torito boot catalog — `add_eltorito(bootcatfile=...)` must be called
   explicitly so a boot catalog exists on disk (the file `_patch_hybrid`
   scans for and OVMF's CD boot path loads), mirroring the xorriso path's
   `-eltorito-alt-boot -e esp.img -no-emul-boot`.

It then asserts the OUTPUT ISO contract that OVMF and dd/USB boot depend
on: an El Torito Boot Record Volume Descriptor pointing at a VALID boot
catalog (validation entry + bootable no-emul initial entry targeting the
ESP), plus a hybrid MBR (protective GPT partition type 0xEE) and GPT
(EFI PART header at sector 1, backup at the last sector, ESP partition
entry). A minimal 4-sector ESP is used so the image is tiny — this pins
the `_patch_hybrid` 1 MiB floor that prevents the backup GPT from
overwriting the low-LBA boot catalog (a real bug on small images).

BOTH builders share one gate: the same contract mixin (`IsoContractMixin`)
runs against the pycdlib-produced ISO and — when xorriso is available —
the xorriso-produced ISO, so a fix to one builder's output cannot silently
diverge from the other's. The cross-builder invariant is that the GPT ESP
partition start (512-byte LBA) equals the El Torito initial entry's ESP
extent (2048-byte LBA) x 4: the catalog and the partition table must point
at the SAME bytes for both builders (for the pycdlib builder with the
4-sector fake ESP that resolves to 104). The xorriso leg skips cleanly
when xorriso is absent (locally or in CI); it goes live the moment it
exists.

Run:  python3 tests/test_make_iso_pycdlib.py
"""

import os
import shutil
import struct
import sys
import tempfile
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPO_ROOT, "scripts"))

import make_iso  # noqa: E402

EFI_ESP_GUID = bytes.fromhex("28732AC11FF8D211BA4B00A0C93EC93B")
BOOT_SIG = bytes([0xEB, 0x3C])  # first two bytes of the fake ESP BPB


def _write_fake_esp(esp_path, esp_size):
    """Minimal FAT-ish blob with a boot signature (BPB + 55 AA)."""
    with open(esp_path, "wb") as f:
        f.write(bytes([0xEB, 0x3C, 0x90]))
        f.write(bytes([0x00]) * 509)
        f.write(bytes([0x55, 0xAA]))
        f.write(bytes([0x00]) * (esp_size - 512))


def build_iso_pycdlib(esp_size):
    """Run the real pycdlib pipeline with a fake ESP; return iso bytes."""
    tmp = tempfile.mkdtemp(prefix="iso_test_")
    try:
        esp_path = os.path.join(tmp, "esp.img")
        iso_path = os.path.join(tmp, "out.iso")
        _write_fake_esp(esp_path, esp_size)
        ok = make_iso.create_iso_pycdlib(esp_path, iso_path, "0.0.0")
        if not ok:
            raise AssertionError("create_iso_pycdlib returned False")
        with open(iso_path, "rb") as f:
            return f.read()
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def build_iso_xorriso(esp_size):
    """Run the real xorriso pipeline with a fake ESP; return iso bytes.

    The caller must skip first when `make_iso.find_xorriso()` is None —
    a build failure here is a real failure, not a skip.
    """
    tmp = tempfile.mkdtemp(prefix="iso_xor_")
    try:
        esp_path = os.path.join(tmp, "esp.img")
        iso_path = os.path.join(tmp, "out.iso")
        _write_fake_esp(esp_path, esp_size)
        ok = make_iso.create_iso_xorriso(esp_path, iso_path, "0.0.0")
        if not ok:
            raise AssertionError("create_iso_xorriso returned False")
        with open(iso_path, "rb") as f:
            return f.read()
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


class IsoContractMixin:
    """The boot-catalog + hybrid contract shared by BOTH ISO builders.

    Subclasses set `self.iso` in setUpClass (a pycdlib or xorriso product);
    every assertion here must hold for both builders, so a fix to one
    builder's output cannot silently diverge from the other's. Both go
    through the same `_patch_hybrid`, so the MBR/GPT structure is shared;
    the El Torito catalog is written by each builder (pycdlib's
    `add_eltorito(bootcatfile=...)` vs xorriso's `-eltorito-alt-boot`),
    and `_patch_hybrid` rewrites the BRVD catalog pointer from the ISO
    root-directory record in both cases.
    """

    iso = None  # set by the concrete class's setUpClass

    def _catalog(self):
        cat_lba = struct.unpack_from("<I", self.iso, 17 * 2048 + 71)[0]
        return cat_lba, self.iso[cat_lba * 2048 : cat_lba * 2048 + 64]

    def test_iso_has_eltorito_boot_catalog(self):
        # Boot Record Volume Descriptor lives at sector 17 (2048-byte units).
        brvd = self.iso[17 * 2048 : 17 * 2048 + 7]
        self.assertEqual(brvd, bytes([0x00]) + b"CD001" + bytes([0x01]),
                         "BRVD magic/type missing")
        cat_lba, cat = self._catalog()
        self.assertGreater(cat_lba, 0, "boot catalog LBA not recorded")

        # Validation entry: header ID 1, platform x86 (0x00, pycdlib) or
        # EFI (0xEF, xorriso for `-e` boot images -- both valid for OVMF),
        # key bytes 55 AA at offsets 30-31 (byte 28-29 hold the
        # checksum-complement).
        self.assertEqual(cat[0], 0x01, "validation entry header ID != 1")
        self.assertIn(cat[1], (0x00, 0xEF),
                      "validation platform neither x86 (0x00) nor EFI (0xEF)")
        self.assertEqual(cat[30:32], bytes([0x55, 0xAA]),
                         "validation key bytes missing")

        # Initial entry (32 bytes into the catalog): bootable 0x88, media
        # no-emul 0, points at the ESP.
        init = cat[32:64]
        self.assertEqual(init[0], 0x88, "initial entry not marked bootable")
        self.assertEqual(init[1], 0x00, "initial entry not no-emulation")
        esp_lba = struct.unpack_from("<I", init, 8)[0]
        self.assertGreater(esp_lba, 0, "catalog ESP LBA is zero")
        # The ESP extent must actually contain the ESP data (boot sig).
        self.assertEqual(self.iso[esp_lba * 2048 : esp_lba * 2048 + 2],
                         BOOT_SIG, "ESP LBA wrong")

    def test_iso_has_protective_mbr(self):
        self.assertEqual(self.iso[510:512], bytes([0x55, 0xAA]),
                         "MBR boot signature missing")
        # Protective GPT partition: type 0xEE at offset 446+4.
        self.assertEqual(self.iso[450], 0xEE, "MBR partition type != 0xEE (GPT)")

    def test_iso_has_gpt(self):
        total = len(self.iso) // 512
        self.assertGreater(total, 64, "ISO too small to be a valid hybrid")
        self.assertEqual(self.iso[512:520], b"EFI PART", "GPT header magic missing")
        # Backup GPT header at the last sector.
        self.assertEqual(
            self.iso[(total - 1) * 512 : (total - 1) * 512 + 8],
            b"EFI PART",
            "backup GPT header missing at last sector",
        )
        # First partition entry at LBA 2 must be the EFI System Partition.
        self.assertEqual(self.iso[1024:1040], EFI_ESP_GUID,
                         "partition entry type != ESP GUID")

    def test_read_gpt_parts_resolves_esp(self):
        # The hybrid's purpose: the tool's own extraction path must locate
        # the ESP partition in the patched ISO (dd/USB and extract_esp
        # depend on it). Cross-builder invariant: the GPT ESP start LBA
        # (512-byte units) must equal the El Torito initial entry's ESP
        # extent (2048-byte units) x4 — the catalog and the partition
        # table must point at the SAME bytes, for BOTH builders. (For the
        # pycdlib builder with the 4-sector fake ESP that resolves to 104.)
        _, cat = self._catalog()
        init_esp_lba = struct.unpack_from("<I", cat, 32 + 8)[0]
        tmp = tempfile.mkdtemp(prefix="iso_gpt_")
        try:
            p = os.path.join(tmp, "roundtrip.iso")
            with open(p, "wb") as f:
                f.write(self.iso)
            parts = make_iso.read_gpt_parts(p)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)
        esp = [pt for pt in parts if pt.get("is_esp")]
        self.assertEqual(len(esp), 1, "read_gpt_parts found no ESP partition")
        self.assertEqual(esp[0]["start"], init_esp_lba * 4,
                         "GPT ESP start != El Torito initial-entry LBA x4")

    def test_catalog_survives_backup_gpt(self):
        # The 1 MiB floor regression pin: with a tiny ESP the backup GPT
        # (written across the final 33 sectors) must NOT overwrite the
        # low-LBA boot catalog. Without the floor this reads zeros and the
        # validation entry assertion fails.
        cat_lba, cat = self._catalog()
        self.assertEqual(cat[0], 0x01,
                         "catalog clobbered by backup GPT")
        self.assertEqual(cat[30:32], bytes([0x55, 0xAA]),
                         "catalog key clobbered by backup GPT")
        self.assertGreater(cat_lba, 0, "boot catalog LBA not recorded")


class TestPycdlibIso(IsoContractMixin, unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        try:
            import pycdlib  # noqa: F401
        except ImportError:
            raise unittest.SkipTest("pycdlib not installed")
        cls.iso = build_iso_pycdlib(4 * 512)  # minimal ESP exercises the floor

    def test_pycdlib_can_reopen(self):
        # The strongest validity check: pycdlib's own parser must be able to
        # re-open the patched hybrid (it validates the boot catalog).
        import pycdlib

        tmp = tempfile.mkdtemp(prefix="iso_reopen_")
        try:
            p = os.path.join(tmp, "reopen.iso")
            with open(p, "wb") as f:
                f.write(self.iso)
            iso = pycdlib.PyCdlib()
            iso.open(p)
            iso.close()
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


class TestXorrisoIso(IsoContractMixin, unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Same fake ESP, same contract — built by the xorriso pipeline.
        # Skips cleanly when xorriso is absent (local Windows box, CI
        # without it); a build failure with xorriso present is a FAIL.
        if make_iso.find_xorriso() is None:
            raise unittest.SkipTest("xorriso not available")
        cls.iso = build_iso_xorriso(4 * 512)


if __name__ == "__main__":
    unittest.main()
