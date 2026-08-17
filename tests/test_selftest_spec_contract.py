#!/usr/bin/env python3
"""Host-runnable contract test pinning the Option 1 selftest spec
(ade/docs/kernel-gui-selftest-spec.md) against the live kernel's TAP
framework and the FUTURE gui_tests.rs, so the spec cannot drift while the
kernel rewrite is in flight:

1. The spec's quoted TAP test names (gui::option1_fallback_forced,
   gui::option1_promotion_maps, gui::option1_renderable) parse cleanly and
   each is named in the spec prose.
2. The spec's framework claims match the LIVE kernel: the
   kernel/src/selftest.rs register(name, TestFn) signature + the
   'ok N - name' TAP ok-line, and kernel/src/tests/memory_tests.rs
   per-module register() shape (the pattern gui_tests.rs mirrors).
3. When the rewrite lands kernel/src/tests/gui_tests.rs, its
   selftest::register("...") call names must equal the spec's names exactly
   (both directions). Until then the check skips with a named message (CI
   checks the kernel out, so CI has teeth the moment the file lands).
4. The cross-links are pinned: kernel-gui-window-fix.md references the spec,
   the spec references the fix doc, and session-lifecycle.md's K3 queue row
   lists the doc link + the three test names.
"""
import io
import os
import re
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ADE_DOCS = os.path.join(REPO_ROOT, "ade", "docs")
SPEC = os.path.join(ADE_DOCS, "kernel-gui-selftest-spec.md")
FIX_DOC = os.path.join(ADE_DOCS, "kernel-gui-window-fix.md")
SL_DOC = os.path.join(ADE_DOCS, "session-lifecycle.md")

EXPECTED_NAMES = [
    "gui::option1_fallback_forced",
    "gui::option1_promotion_maps",
    "gui::option1_renderable",
]

OPTION2_SPEC = os.path.join(ADE_DOCS, "kernel-gui-selftest-spec-option2.md")

# Option 2's selftest family (kernel-gui-selftest-spec-option2.md). Mutually
# exclusive with EXPECTED_NAMES at the create_window fallback site: the
# rewrite registers ONE family (fix doc: "the two kernel options are
# mutually exclusive at the create_window fallback level").
EXPECTED_OPTION2_NAMES = [
    "gui::option2_enomem_forced",
    "gui::option2_create_succeeds_when_room",
]


def _read(p):
    with io.open(p, encoding="utf-8") as fh:
        return fh.read()


def _kernel_root():
    env = os.environ.get("SKYOS_KERNEL_DIR")
    cands = ([env] if env else [])
    parent = os.path.dirname(REPO_ROOT)
    cands += [os.path.join(parent, "SKYIOUS KERNEL"),
              os.path.join(parent, "SKYIOUS-KERNEL"),
              os.path.join(parent, "SKYIOUS_KERNEL")]
    return next((c for c in cands if c and os.path.isfile(
        os.path.join(c, "kernel", "src", "selftest.rs"))), None)


class TestSelftestSpecContract(unittest.TestCase):
    maxDiff = None

    def test_spec_tap_names_parse(self):
        s = _read(SPEC)
        names = re.findall(r'crate::selftest::register\("([^"]+)",', s)
        self.assertEqual(names, EXPECTED_NAMES,
                         "register() names in kernel-gui-selftest-spec.md "
                         "drifted from the three Option 1 TAP tests")
        for n in EXPECTED_NAMES:
            self.assertIn(n, s, "%s missing from the spec prose" % n)
        # The spec must name the framework it targets.
        self.assertIn("kernel/src/selftest.rs", s,
                      "spec no longer names kernel/src/selftest.rs as the "
                      "TAP framework it targets")

    def test_spec_framework_claims_match_live_kernel(self):
        root = _kernel_root()
        if root is None:
            self.skipTest("kernel tree not found (SKYOS_KERNEL_DIR or a "
                          "SKYIOUS KERNEL sibling); CI checks it out, so CI has teeth")
        st = _read(os.path.join(root, "kernel", "src", "selftest.rs"))
        # The exact signature the spec quotes for register().
        self.assertIn("pub fn register(name: &'static str, func: TestFn)", st,
                      "selftest.rs register() signature changed - the spec "
                      "quotes it verbatim (kernel/src/selftest.rs)")
        self.assertIn("pub type TestFn = fn() -> Result<(), &'static str>;", st,
                      "selftest.rs TestFn changed - the spec quotes "
                      "fn() -> Result<(), &'static str>")
        # The TAP ok-line format the spec says run_all() prints (and the CI
        # grep gate consumes).
        self.assertIn('"ok {} - {}\\n"', st,
                      "selftest.rs no longer prints 'ok N - name' TAP lines - "
                      "the CI grep and the spec's claim both depend on it")
        # The per-module register() shape the spec's gui_tests.rs mirrors.
        mt = _read(os.path.join(root, "kernel", "src", "tests", "memory_tests.rs"))
        self.assertIn("pub fn register()", mt)
        self.assertIn('crate::selftest::register("', mt,
                      "memory_tests.rs lost the per-module register() shape "
                      "the spec's gui_tests.rs wiring mirrors")

    def test_gui_tests_register_matches_spec(self):
        root = _kernel_root()
        if root is None:
            self.skipTest("kernel tree not found (SKYOS_KERNEL_DIR or a "
                          "SKYIOUS KERNEL sibling); CI checks it out, so CI has teeth")
        gt = os.path.join(root, "kernel", "src", "tests", "gui_tests.rs")
        if not os.path.isfile(gt):
            self.skipTest("kernel/src/tests/gui_tests.rs not landed yet - "
                          "will assert its register() names match the spec "
                          "the moment the rewrite adds it (CI checks the "
                          "kernel out, so CI has teeth)")
        s = _read(gt)
        names = re.findall(r'register\("([^"]+)",', s)
        self.assertEqual(sorted(names), sorted(EXPECTED_NAMES),
                         "gui_tests.rs register() names no longer match the "
                         "spec's gui::option1_* TAP tests")

    def test_crosslinks_pinned(self):
        fix = _read(FIX_DOC)
        self.assertIn("kernel-gui-selftest-spec.md", fix,
                      "kernel-gui-window-fix.md lost its link to the selftest "
                      "spec (Verification plan, Option 1 code path)")
        self.assertIn("TAP framework wiring", fix,
                      "kernel-gui-window-fix.md's spec reference lost its "
                      "TAP-framework context")
        spec = _read(SPEC)
        self.assertIn("kernel-gui-window-fix.md", spec,
                      "kernel-gui-selftest-spec.md lost its companion link to "
                      "the fix doc")
        sl = _read(SL_DOC)
        self.assertIn("kernel-gui-selftest-spec.md", sl,
                      "session-lifecycle.md K3 queue row lost the spec link")
        for n in EXPECTED_NAMES:
            # Backticked anchor: the K3 row writes the names as `gui::...`,
            # so a renamed token (gui::option1_renderableX) can't satisfy
            # the needle as a substring of itself (prefix-trap class).
            self.assertIn("`%s`" % n, sl,
                          "K3 queue row no longer lists the TAP test %s" % n)

    def test_forced_failure_boot_leg_pinned(self):
        # Second user-visible path (Aug 13, 2026): the spec must keep the
        # forced-failure boot leg — the test-only drain hook that makes
        # login-manager's real 800x600 create run under near-zero free
        # pages — in lockstep with the serial markers it asserts and the
        # fix doc's cross-link back to it. All needles are scoped to the
        # leg section (s[leg_start:]) so a mutation anywhere else in the
        # spec cannot satisfy them (placement-needle class).
        s = _read(SPEC)
        leg_start = s.find("## Second user-visible path")
        self.assertGreater(leg_start, 0,
                           "spec lost the forced-failure boot leg section")
        leg = s[leg_start:]
        # Drain hook contract: flag name, hold semantics, order-9.
        self.assertIn("`SKYOS_DRAIN_BUDDY=<order>`", leg,
                      "leg lost the SKYOS_DRAIN_BUDDY drain-hook flag")
        self.assertIn("drain_order", leg,
                      "leg lost the drain_order helper reference")
        self.assertIn("core::mem::forget", leg,
                      "leg lost the hold-leak semantics (mem::forget)")
        # The two serial markers, backticked, with the near-zero magnitude
        # and the absent-success leg.
        self.assertIn("`[login] mem free=N pages`", leg)
        self.assertIn("`[login] failed to create window`", leg)
        self.assertIn("near zero", leg,
                      "leg lost the near-zero magnitude assertion")
        self.assertIn("ABSENT", leg,
                      "leg lost the window-created ABSENT leg")
        # Marker order: mem free=N (1) must precede failed to create (2).
        self.assertLess(leg.find("mem free=N"), leg.find("failed to create window"),
                        "leg no longer lists the mem marker before the "
                        "failed-to-create marker (serial order contract)")
        # Evidence-table mapping + give-up harness bridge.
        self.assertIn("Option 2 + 2b", leg,
                      "leg lost the persistent->Option 2 mapping row")
        self.assertIn("qemu_giveup_boot.exp", leg,
                      "leg lost the give-up harness bridge reference")
        self.assertIn("`mem_readings`", leg,
                      "leg lost the mem_readings series reference")
        # The fix doc cross-links back to the leg.
        fix = _read(FIX_DOC)
        self.assertIn("Second user-visible path", fix,
                      "kernel-gui-window-fix.md lost its cross-link to the "
                      "boot leg")
        self.assertIn("`SKYOS_DRAIN_BUDDY=<order>`", fix)

    def test_option2_spec_pinned(self):
        # Option 2's selftest spec (Aug 13, 2026): the honest -ENOMEM
        # variant must keep its TAP names, the kernel errno contract, the
        # userspace 2b markers, and the cross-links in lockstep — the
        # counterweight to the Option 1 pins above. The two option families
        # are mutually exclusive, so a spec edit that drifts one family
        # cannot hide behind the other's pins.
        s = _read(OPTION2_SPEC)
        names = re.findall(r'crate::selftest::register\("([^"]+)",', s)
        self.assertEqual(names, EXPECTED_OPTION2_NAMES,
                         "kernel-gui-selftest-spec-option2.md register() "
                         "names drifted from the two Option 2 TAP tests")
        for n in EXPECTED_OPTION2_NAMES:
            self.assertIn(n, s, "%s missing from the Option 2 spec prose" % n)
        # Kernel errno contract: the exact expression the Option 2 hunk
        # returns, and the mutual-exclusion note.
        self.assertIn("errno::Errno::ENOMEM as u64", s,
                      "Option 2 spec lost the -ENOMEM-as-u64 return contract")
        self.assertIn("mutually exclusive", s,
                      "Option 2 spec lost the option1/option2 exclusivity note")
        self.assertIn("kernel/src/selftest.rs", s,
                      "Option 2 spec no longer names the TAP framework")
        # Userspace half (Option 2b): the marker, the exit-code constant, and
        # the return site.
        self.assertIn("`[login] window create failed: Out of memory`", s,
                      "Option 2 spec lost the Out-of-memory serial marker")
        self.assertIn("EXIT_WINDOW_CREATE_FAILED", s,
                      "Option 2 spec lost the EXIT_WINDOW_CREATE_FAILED "
                      "exit-code contract")
        self.assertIn("return EXIT_WINDOW_CREATE_FAILED", s,
                      "Option 2 spec lost the non-zero return arm")
        # Cross-pins: the fix doc carries the same marker in its Option 2b
        # hunk and links to the spec; the queue row links to the spec too.
        fix = _read(FIX_DOC)
        self.assertIn("Out of memory", fix,
                      "kernel-gui-window-fix.md Option 2b hunk lost the "
                      "Out-of-memory marker the spec pins")
        self.assertIn("kernel-gui-selftest-spec-option2.md", fix,
                      "kernel-gui-window-fix.md lost its link to the Option "
                      "2 selftest spec")
        sl = _read(SL_DOC)
        self.assertIn("kernel-gui-selftest-spec-option2.md", sl,
                      "session-lifecycle.md lost the K3-alt queue row link "
                      "to the Option 2 selftest spec")


if __name__ == "__main__":
    unittest.main()
