#!/usr/bin/env python3
"""Host-runnable contrast gate for the keyboard-focus fills (no QEMU).

The focused surface everywhere (taskbar buttons, start-menu rows, window
Close/Minimize chrome, tray entries, notification rows) draws the lighter
`accent_light` blue with the white `on_accent` text. taskbar.rs:127-130
documents that pairing as ~3.42:1 — deliberately above the WCAG 3:1
UI-component floor but below the 4.5:1 AA the indigo hover fill gets (the
distinct focus hue is the tradeoff). This suite makes that a COMPUTED gate:
it parses `accent_light`/`on_accent` out of BOTH theme constructors in
libsarga/src/theme.rs, runs the real WCAG relative-luminance/contrast
formula (the same numbers the Rust selftest's Newton fifth-root produces),
and asserts the ratio stays above the floor in both themes — so a palette
edit that dims the focused blue fails CI here with the actual ratio, before
any boot. It also pins the Rust `test_theme_contrast` selftest to the same
pair (the two must not drift) and self-pins its own host-tests CI step.

Run:  python3 tests/test_theme_contrast.py
"""
import os
import re
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
THEME_RS = os.path.join(REPO_ROOT, "libsarga", "src", "theme.rs")
SELFTEST_RS = os.path.join(REPO_ROOT, "ade", "src", "util", "testing", "theme.rs")
TASKBAR_RS = os.path.join(REPO_ROOT, "ade", "src", "core", "taskbar.rs")
CI_YML = os.path.join(REPO_ROOT, ".github", "workflows", "ci.yml")

# The documented minimum: WCAG 1.4.11 Non-text Contrast (UI components)
# requires 3:1; the focused fill is a component-state indicator, so the
# floor is 3.0. Kept as the single literal so a future threshold change
# has one place to edit (and the negative legs one place to attack).
UI_COMPONENT_FLOOR = 3.0


def _read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


# --- Pure WCAG port (mirrors ade/src/util/testing/theme.rs) ---
def linearize(ch):
    """sRGB channel -> linear 0..=1 (WCAG piecewise)."""
    c = ch / 255.0
    if c <= 0.03928:
        return c / 12.92
    return ((c + 0.055) / 1.055) ** 2.4


def luminance(color):
    """WCAG relative luminance of an 0xRRGGBB (or 0xAARRGGBB) color."""
    r = (color >> 16) & 0xFF
    g = (color >> 8) & 0xFF
    b = color & 0xFF
    return 0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)


def contrast(fg, bg):
    """WCAG contrast ratio between two colors (order-independent)."""
    a, b = luminance(fg), luminance(bg)
    if a < b:
        a, b = b, a
    return (a + 0.05) / (b + 0.05)


# --- Source extraction (the palette is the contract, not a hardcoded copy) ---
def _constructor_body(src, name):
    """Body of `pub fn <name>() -> Self { ... }` from theme.rs."""
    m = re.search(r"pub fn %s\(\) -> Self \{(.*?)\n    \}" % name, src, re.S)
    assert m, "constructor %s not found in theme.rs" % name
    return m.group(1)


def _field(body, field):
    m = re.search(r"%s:\s*(0x[0-9A-Fa-f]+)" % field, body)
    assert m, "field %s not found in constructor" % field
    return int(m.group(1), 16)


def palette(theme_name):
    """(accent_light, on_accent) for 'light' or 'dark' from source."""
    src = _read(THEME_RS)
    body = _constructor_body(src, theme_name)
    return _field(body, "accent_light"), _field(body, "on_accent")


class FocusedFillContrastGate(unittest.TestCase):
    """The computed gate: both themes, real WCAG math, real palette values."""

    def test_light_focused_fill_above_ui_floor(self):
        accent_light, on_accent = palette("light")
        ratio = contrast(on_accent, accent_light)
        self.assertGreaterEqual(
            ratio, UI_COMPONENT_FLOOR,
            "light focused fill fell below the 3:1 UI-component floor: "
            "on_accent 0x%08X on accent_light 0x%08X is %.2f:1" % (
                on_accent, accent_light, ratio),
        )

    def test_dark_focused_fill_above_ui_floor(self):
        accent_light, on_accent = palette("dark")
        ratio = contrast(on_accent, accent_light)
        self.assertGreaterEqual(
            ratio, UI_COMPONENT_FLOOR,
            "dark focused fill fell below the 3:1 UI-component floor: "
            "on_accent 0x%08X on accent_light 0x%08X is %.2f:1" % (
                on_accent, accent_light, ratio),
        )

    def test_documented_ratio_matches_computed(self):
        # taskbar.rs documents ~3.42:1 for the focused pair. The computed
        # value must agree with the doc claim (within rounding) so the
        # comment and the math can't drift apart.
        accent_light, on_accent = palette("light")
        ratio = contrast(on_accent, accent_light)
        self.assertAlmostEqual(ratio, 3.42, delta=0.05,
                               msg="focused fill ratio %.2f:1 no longer matches "
                                   "the documented ~3.42:1" % ratio)


class FocusedFillSourcePins(unittest.TestCase):
    """The source/documentation side: the floor is documented, the Rust
    selftest covers the pair, and this suite is wired into CI."""

    def setUp(self):
        self.selftest = _read(SELFTEST_RS)
        self.taskbar = _read(TASKBAR_RS)

    def test_rust_selftest_checks_the_pair(self):
        # The QEMU-side theme audit must pin the same focused pair — the
        # host gate and the in-boot selftest share the contract. If the
        # Rust check is ever dropped, this fails.
        self.assertIn("on_accent on accent_light (focused)", self.selftest,
                      "Rust test_theme_contrast no longer checks the focused pair")
        self.assertIn("theme.accent_light,", self.selftest,
                      "Rust selftest no longer feeds accent_light into the check")

    def test_floor_is_documented_in_taskbar(self):
        # The "documented minimum" the gate enforces must stay documented at
        # the draw site that names it (3:1 UI-component floor, ~3.42:1).
        self.assertIn("3:1 UI-component floor", self.taskbar,
                      "taskbar.rs lost the 3:1 floor documentation")
        self.assertIn("accent_light", self.taskbar,
                      "taskbar.rs no longer references accent_light")

    def test_host_tests_step_wired(self):
        # This file must be run by the host-tests job (the thread pattern:
        # a contract suite that is never wired into CI catches nothing). A
        # future edit that drops the step fails the suite, not just CI.
        ci = _read(CI_YML)
        self.assertIn(
            "python3 tests/test_theme_contrast.py", ci,
            "host-tests job lost the theme contrast step",
        )


if __name__ == "__main__":
    unittest.main()
