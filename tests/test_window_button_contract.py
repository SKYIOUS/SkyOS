#!/usr/bin/env python3
"""Host-runnable unit tests for the window chrome render contract (no QEMU).

Pins the draw-side of the focus light: `ade/src/core/window.rs`'s
`window_button_face` (the one interaction union both chrome controls share —
pressed > focused > hover > base. The chrome controls keep their SEMANTIC
fill under focus — the close red / the minimize white wash — and mark
keyboard focus with an accent_light ring around the control, so focus is
visually distinct from pointer hover without erasing the close/minimize
color meaning (the same accent_light hue the taskbar/menu focus fills
use) and `window_button_from_label` (the label -> control reverse map). The
union was inline in the Close/Minimize draws; it now lives as a pure
function over three booleans so this file can port it exactly the way
`test_tooltip_contract.py` ports `format_tooltip`, and a render-contract
regression fails CI before any boot.

The face semantics are a UI contract: pressed is deliberately pointer-only
(hover && mouse_down) — the keyboard-focused state gets the Focused face
but never Presses, matching the taskbar buttons. The label
mapping is also a UI
contract: the QEMU tree-stamping/activation/focus-resolution sites all key
off the same "Close"/"Minimize" strings, so a rename trips here first.

Run:  python3 tests/test_window_button_contract.py
"""
import os
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WINDOW_RS = os.path.join(REPO_ROOT, "ade", "src", "core", "window.rs")
CI_YML = os.path.join(REPO_ROOT, ".github", "workflows", "ci.yml")


def _read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


# --- Pure ports of window.rs (ade/src/core/window.rs) ---
#
# window_button_face(hover, focused, mouse_down): the interaction union
# every chrome control draw uses to pick its face.
def window_button_face(hover, focused, mouse_down):
    if hover and mouse_down:
        return "Pressed"
    if focused:
        return "Focused"
    if hover:
        return "Hover"
    return "Base"


# window_button_from_label(label): the reverse of the WindowButton stamp —
# the a11y tree stamps "Close"/"Minimize" and activation/focus resolution
# read them back through this map.
def window_button_from_label(label):
    return {"Close": "Close", "Minimize": "Minimize"}.get(label)


class WindowButtonSourceContract(unittest.TestCase):
    def setUp(self):
        self.window = _read(WINDOW_RS)

    def test_face_union_is_a_pure_function(self):
        # The whole point: the union lives once as a pure 3-boolean function
        # beside WinInteraction, not inline in either control's draw.
        self.assertIn("pub(crate) fn window_button_face(", self.window,
                      "window_button_face missing from window.rs")
        self.assertIn("pub(crate) enum WindowButtonFace {", self.window,
                      "WindowButtonFace missing from window.rs")
        self.assertIn("    Focused,", self.window,
                      "WindowButtonFace lost its Focused variant")

    def test_pressed_wins_then_focus_then_hover_then_base(self):
        # Precedence order is the contract: pressed (hover && mouse_down)
        # must be decided first, then the focused face, then the hover
        # light, then base. Focus beating hover means a ring + pointer on
        # the same control shows the accent_light focus, not the indigo
        # hover — the taskbar rule. Assert the arms appear in order.
        i = self.window.find("fn window_button_face(")
        self.assertNotEqual(i, -1)
        body = self.window[i:i + 500]
        j_pressed = body.find("hover && mouse_down")
        j_focused = body.find("else if focused {")
        j_hover = body.find("else if hover {")
        self.assertNotEqual(j_pressed, -1, "pressed arm missing from union")
        self.assertNotEqual(j_focused, -1, "focused arm missing from union")
        self.assertNotEqual(j_hover, -1, "hover arm missing from union")
        self.assertLess(j_pressed, j_focused,
                        "pressed arm must be decided before the focused arm")
        self.assertLess(j_focused, j_hover,
                        "focused arm must be decided before the hover arm")

    def test_both_controls_route_through_the_union(self):
        # Close and Minimize must share the union — re-inlining either site
        # trips here before any QEMU boot.
        self.assertIn("window_button_face(hover_close, focused_close, ix.mouse_down)",
                      self.window, "Close draw re-inlined its face union")
        self.assertIn("window_button_face(hover_min, focused_min, ix.mouse_down)",
                      self.window, "Minimize draw re-inlined its face union")
        # The exact color mapping is the contract: the chrome controls keep
        # their SEMANTIC fill under focus — the close red (WIN_CLOSE_HOVER)
        # and the minimize white wash — and mark focus with an accent_light
        # ring drawn after the fill, so keyboard focus stays visually
        # distinct from pointer hover without erasing the close/minimize
        # color meaning. The Focused face stays distinct from Hover in the
        # union; the draws merge their fill and add the ring on Focused.
        self.assertIn(
            'WindowButtonFace::Focused | WindowButtonFace::Hover => {\n            libsarga::theme::colors::WIN_CLOSE_HOVER',
            self.window,
            "Close focused arm lost its semantic red fill",
        )
        self.assertIn(
            'WindowButtonFace::Focused | WindowButtonFace::Hover => {\n            canvas.draw_rect_alpha(min_x, close_y, min.w, min.h, 0x35FFFFFF)',
            self.window,
            "Minimize focused arm lost its white wash",
        )
        self.assertIn("if focused_close {",
                      self.window, "Close focused ring removed")
        self.assertIn("if focused_min {",
                      self.window, "Minimize focused ring removed")
        self.assertIn("apply_alpha(theme.accent_light, aw.flags.opacity)",
                      self.window, "focused ring no longer uses accent_light")
        # The white wash is the SAME under hover and focus — the ring is the
        # only focus marker on the minimize glyph, which must stay
        # theme.text (on_accent white would vanish on the wash).
        self.assertIn("apply_alpha(theme.text, aw.flags.opacity)",
                      self.window, "minimize glyph no longer theme.text")

    def test_label_map_still_matches_the_stamp(self):
        # The a11y tree stamps these exact strings; the resolution reads them
        # back. Both arms must exist in the source map.
        self.assertIn('"Close" => Some(WindowButton::Close)', self.window)
        self.assertIn('"Minimize" => Some(WindowButton::Minimize)', self.window)
        self.assertIn('_ => None', self.window)

    def test_host_tests_step_wired(self):
        # This file must be run by the host-tests job (the thread pattern:
        # a contract suite that is never wired into CI catches nothing). A
        # future edit that drops the step fails the suite, not just CI.
        ci = _read(CI_YML)
        self.assertIn(
            "python3 tests/test_window_button_contract.py", ci,
            "host-tests job lost the window button contract step",
        )


class WindowButtonFacePort(unittest.TestCase):
    """Behavioral tests on the port — the full (hover, focused, mouse_down)
    truth table plus the label mapping, mirroring what the QEMU selftests
    exercise through the snapshot."""

    def test_base_state(self):
        # Nothing touching the control: no light.
        self.assertEqual(window_button_face(False, False, False), "Base")
        # Mouse down over empty chrome (not this control): still Base.
        self.assertEqual(window_button_face(False, False, True), "Base")

    def test_focused_only_lights_focused(self):
        # Keyboard ring on the control, mouse nowhere: the Focused face —
        # the accent_light distinction, NOT the hover light.
        self.assertEqual(window_button_face(False, True, False), "Focused")

    def test_focused_never_presses(self):
        # The pointer-only pressed contract: focus + mouse-down elsewhere is
        # still Focused, never Pressed (a keyboard user can't press).
        self.assertEqual(window_button_face(False, True, True), "Focused")

    def test_hover_only_lights_hover(self):
        self.assertEqual(window_button_face(True, False, False), "Hover")

    def test_pressed_requires_hover_and_mouse_down(self):
        self.assertEqual(window_button_face(True, False, True), "Pressed")
        # Hover + focus + mouse down is still the pointer press.
        self.assertEqual(window_button_face(True, True, True), "Pressed")

    def test_focus_beats_hover(self):
        # Ring and pointer on the same control: focus wins (the ring is the
        # active mode), so the accent_light face shows, not the hover.
        self.assertEqual(window_button_face(True, True, False), "Focused")

    def test_resting_base(self):
        self.assertEqual(window_button_face(False, False, False), "Base")

    def test_label_resolution(self):
        self.assertEqual(window_button_from_label("Close"), "Close")
        self.assertEqual(window_button_from_label("Minimize"), "Minimize")
        # Non-chrome labels and empty labels resolve to None (no control).
        self.assertIsNone(window_button_from_label("Maximize"))
        self.assertIsNone(window_button_from_label(""))
        self.assertIsNone(window_button_from_label("Settings"))


if __name__ == "__main__":
    unittest.main()
