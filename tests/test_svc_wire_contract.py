#!/usr/bin/env python3
"""Host-runnable source-contract test pinning the SVC wire table in
libsarga/src/ipc.rs — the canonical 5-entry IPC service catalog (facility
audit F5), so a future renumber or re-add fails CI before any QEMU boot.

The F5 cleanup removed four unserved services (Launcher/Session/Theme/
Power) and RENUMBERED the survivors — FILE_DIALOG 3->2, SETTINGS 4->3,
WINDOW 6->4 — because the wire ids are the wire protocol. The current
table is the contract:

    SVC_CLIPBOARD    = 0
    SVC_NOTIFICATION = 1
    SVC_FILE_DIALOG  = 2
    SVC_SETTINGS     = 3
    SVC_WINDOW       = 4

ade's ServiceId registry maps to these exact constants in to_wire() /
from_wire() (ade/src/ipc/registry.rs), so a drift on either side breaks
the wire round-trip. These tests pin both sides: the libsarga table, the
absence of the four removed constants, and the registry wiring.

Run:  python3 tests/test_svc_wire_contract.py
"""
import os
import re
import sys
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from scan_rust import strip_rust  # noqa: E402

LIBSARGA_IPC_RS = os.path.join(REPO_ROOT, "libsarga", "src", "ipc.rs")
REGISTRY_RS = os.path.join(REPO_ROOT, "ade", "src", "ipc", "registry.rs")

# The wire table as it must appear in libsarga/src/ipc.rs, name -> value.
WIRE_TABLE = {
    "SVC_CLIPBOARD": 0,
    "SVC_NOTIFICATION": 1,
    "SVC_FILE_DIALOG": 2,
    "SVC_SETTINGS": 3,
    "SVC_WINDOW": 4,
}

# The four F5-removed services must stay out of the wire table.
REMOVED = ("SVC_LAUNCHER", "SVC_SESSION", "SVC_THEME", "SVC_POWER")

# libsarga SVC_ name -> ade ServiceId variant spelling. Explicit, because
# the wire-prefix-to-variant transform is not a simple case change:
# SVC_FILE_DIALOG -> FileDialog (not FILE_DIALOG).
SVC_TO_VARIANT = {
    "SVC_CLIPBOARD": "Clipboard",
    "SVC_NOTIFICATION": "Notification",
    "SVC_FILE_DIALOG": "FileDialog",
    "SVC_SETTINGS": "Settings",
    "SVC_WINDOW": "Window",
}


def _read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def _wire_defs(code):
    """All SVC_* constant definitions: {NAME: value}."""
    return {
        m.group(1): int(m.group(2))
        for m in re.finditer(r"pub const (SVC_\w+): u8 = (\d+);", code)
    }


class TestSvcWireTable(unittest.TestCase):
    """libsarga/src/ipc.rs defines exactly the five constants, in order."""

    def test_wire_table_exact(self):
        code = strip_rust(_read(LIBSARGA_IPC_RS))
        self.assertEqual(
            _wire_defs(code), WIRE_TABLE,
            "libsarga SVC wire table drifted from the 5-entry contract "
            "(a renumber or re-add changes the wire protocol)",
        )

    def test_removed_constants_stay_absent(self):
        code = strip_rust(_read(LIBSARGA_IPC_RS))
        for name in REMOVED:
            self.assertNotIn(
                "pub const %s:" % name, code,
                "F5-removed service %s was re-added to the wire table" % name,
            )


class TestRegistryWiring(unittest.TestCase):
    """ade's ServiceId registry maps 1:1 to the libsarga constants."""

    def test_to_wire_arms_match_table(self):
        code = strip_rust(_read(REGISTRY_RS))
        start = code.index("fn to_wire")
        end = code.index("fn from_wire")
        region = code[start:end]
        for name, _value in WIRE_TABLE.items():
            variant = SVC_TO_VARIANT[name]
            self.assertIn(
                "ServiceId::%s => libsarga::ipc::%s," % (variant, name),
                region,
                "registry to_wire lost its %s arm (wire id %s)" % (name, name),
            )

    def test_from_wire_round_trips_table(self):
        code = strip_rust(_read(REGISTRY_RS))
        start = code.index("fn from_wire")
        region = code[start:]
        for name, _value in WIRE_TABLE.items():
            variant = SVC_TO_VARIANT[name]
            self.assertIn(
                "libsarga::ipc::%s => Some(ServiceId::%s)," % (name, variant),
                region,
                "registry from_wire lost its %s arm" % name,
            )
        self.assertIn(
            "_ => None,",
            region,
            "from_wire lost its fallthrough: an unknown wire id must be "
            "rejected, not silently mapped",
        )


if __name__ == "__main__":
    unittest.main()
