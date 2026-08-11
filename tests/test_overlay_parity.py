#!/usr/bin/env python3
"""Host-runnable source-contract pin: dismiss_overlays and handle_click agree.

The Rust selftest test_a11y_overlay_mouse_keyboard_parity (QEMU) exercises
the behavior; this file runs on the host without a boot and pins the
STRUCTURE that makes the parity hold, so a reordering cannot regress the
contract before any QEMU run:

  * the overlay flags handle_click checks BEFORE the taskbar branch equal
    the flags dismiss_overlays closes (set equality -- exactly the six);
  * the context-menu check precedes the taskbar branch in handle_click (it
    historically ran after, so a taskbar click with the menu open acted
    beneath it while keyboard Enter dismissed it);
  * the a11y Enter arm calls dismiss_overlays before activate_a11y_node
    (dismiss-before-acting on the keyboard side).

Run:  python3 tests/test_overlay_parity.py
"""
import os
import re
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DESKTOP_RS = os.path.join(REPO_ROOT, "ade", "src", "core", "desktop.rs")

OVERLAYS = (
    "start_menu",
    "context_menu",
    "settings",
    "settings_app",
    "task_manager",
    "about_state",
)


def _read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def _fn_body(src, fn):
    start = src.index("fn %s(" % fn)
    brace = src.index("{", start)
    depth = 1
    i = brace + 1
    while depth:
        ch = src[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        i += 1
    return src[start:i]


# Guard-precise overlay detection: the DISMISSAL GUARDS are `if
# self.<field>.open` (panels/menu) and `if let Some(cm) =
# self.context_menu` / `if self.context_menu.is_some()` (context menu).
# Incidental references (e.g. `self.context_menu = None` inside the
# settings Close arm) must NOT count, or a context-menu block moved below
# the taskbar would hide inside them and the set pin would never fire.
_GUARD_RE = re.compile(
    r"if self\.(\w+)\.open\b|"
    r"if let Some\(cm\) = self\.context_menu|"
    r"if self\.context_menu\.is_some\(\)"
)


def _overlay_refs(body):
    """Set of overlay fields whose DISMISSAL GUARD appears in a body."""
    # Drop // comments first: prose like "if self.settings.open were here"
    # inside a comment must not count as a guard reference.
    body = re.sub(r"//[^\n]*", "", body)
    found = set()
    for m in _GUARD_RE.finditer(body):
        if m.group(1):
            found.add(m.group(1))
        else:
            found.add("context_menu")
    return found


class OverlayParityContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.src = _read(DESKTOP_RS)
        cls.click = _fn_body(cls.src, "handle_click")
        cls.dismiss = _fn_body(cls.src, "dismiss_overlays")
        cls.a11y = _fn_body(cls.src, "handle_a11y_key")

    def test_handle_click_pre_taskbar_set_equals_dismiss_set(self):
        # The overlay flags checked before the taskbar branch (the region up
        # to `if my >= taskbar_y`) must be exactly the set dismiss_overlays
        # closes -- and exactly the known six (the fixed list is the
        # tripwire: a seventh overlay added on both sides fails here until
        # the list is updated, mirroring the keymap-count pins).
        pre_taskbar = self.click[: self.click.index("if my >= taskbar_y")]
        click_set = _overlay_refs(pre_taskbar)
        dismiss_set = _overlay_refs(self.dismiss)
        self.assertEqual(
            click_set,
            dismiss_set,
            "handle_click pre-taskbar overlays %s != dismiss_overlays %s"
            % (sorted(click_set), sorted(dismiss_set)),
        )
        self.assertEqual(
            click_set,
            set(OVERLAYS),
            "pre-taskbar overlay set drifted from the known six: %s"
            % sorted(click_set),
        )

    def test_context_menu_precedes_taskbar_branch(self):
        # The context menu (the historical parity breaker) must be checked
        # before the taskbar branch so a taskbar click with the menu up
        # only dismisses it, never acts beneath it. Pinned by the GUARD
        # (`if let Some(cm) = self.context_menu`), not the incidental
        # `self.context_menu = None` in the settings Close arm, which would
        # pass even with the block below the taskbar.
        pre_taskbar = self.click[: self.click.index("if my >= taskbar_y")]
        self.assertIn("if let Some(cm) = self.context_menu", pre_taskbar)

    def test_a11y_enter_dismisses_before_activating(self):
        # Keyboard side: the Enter arm calls dismiss_overlays() before
        # activate_a11y_node(), so an Enter on a taskbar node with a modal
        # up is consumed as a dismissal.
        self.assertLess(
            self.a11y.index("self.dismiss_overlays()"),
            self.a11y.index("self.activate_a11y_node("),
            "a11y Enter must dismiss overlays before activating nodes",
        )


if __name__ == "__main__":
    unittest.main()
