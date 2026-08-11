#!/usr/bin/env python3
"""Host-runnable unit tests for ade's pure tooltip label formatter (no QEMU).

Pins the contract of `ade/src/sec/a11y/mod.rs::format_tooltip`: the pure
label-resolution function that produces every hover tooltip (Close/Minimize
window controls, taskbar buttons, start-menu rows, start button, and the
owner/label fallback). The function takes the a11y node, the unified hover
target, and INJECTED lookups — no `Desktop` dependency — so the QEMU boot
suite's tooltip tests (`test_tooltip_role_labels`, `test_tooltip_owner_label`)
can be mirrored here on the host, and a formatting regression fails CI before
any boot.

The label strings are a UI contract: the QEMU selftests and this file must
agree, so both are pinned to the same literals ("Close <t>", "Minimize <t>",
"Switch to <t>", "Restore <t>", "Open Start menu", app descriptions).

Run:  python3 tests/test_tooltip_contract.py
"""
import os
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
A11Y_RS = os.path.join(REPO_ROOT, "ade", "src", "sec", "a11y", "mod.rs")
WINDOW_RS = os.path.join(REPO_ROOT, "ade", "src", "core", "window.rs")


def _read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


# --- Pure port of format_tooltip (ade/src/sec/a11y/mod.rs) ---
#
# Mirrors the Rust exactly: hover targets are represented as (kind, payload)
# tuples and the node as a dict with role/label/owner. The three lookups are
# injected, same as the Rust signature (Python defaults the 4th lookup
# to False so the behavior-port calls stay terse).
def format_tooltip(node, hover, title_of, desc_of, recent_desc_of, minimized_of=None):
    if minimized_of is None:
        minimized_of = lambda wid: False
    if hover is not None:
        kind = hover[0]
        if kind == "Window":
            _, win, btn = hover
            action = "Close" if btn == "Close" else "Minimize"
            return "%s %s" % (action, title_of(win) or "")
        if kind == "TaskbarButton":
            _, wid = hover
            title = title_of(wid) or ""
            if minimized_of(wid):
                return "Restore %s" % title
            return "Switch to %s" % title
        if kind == "StartApp":
            _, i = hover
            return desc_of(i) or ""
        if kind == "StartRecent":
            _, ri = hover
            return recent_desc_of(ri) or ""
        if kind == "StartButton":
            return "Open Start menu"
    # _ arm: owner-stamped title first, then the node label, then the
    # role-name fallback for empty labels.
    if node.get("owner") is not None:
        return title_of(node["owner"]) or node.get("label", "")
    if node.get("label", "") == "":
        return {
            "Taskbar": "Taskbar",
            "StartMenu": "Start Menu",
            "Desktop": "Desktop",
        }.get(node.get("role"), "")
    return node.get("label", "")


class TooltipSourceContract(unittest.TestCase):
    def setUp(self):
        self.a11y = _read(A11Y_RS)
        self.window = _read(WINDOW_RS)

    def test_format_tooltip_is_pure(self):
        # The whole point: no `Desktop` access inside the pure function.
        body = self._fn_body(self.a11y, "pub(crate) fn format_tooltip<'a>(")
        # Purity = no Desktop field access inside the body. (`Desktop` may
        # legitimately appear as a role name in the fallback arm, so assert
        # on the `d.` field-access pattern, not the word.)
        self.assertNotIn("d.", body, "format_tooltip must not touch Desktop")

    def test_signature_injects_lookups(self):
        self.assertIn(
            "fn format_tooltip<'a>(\n    node: &A11yNode,\n    hover: Option<HoverTarget>,",
            self.a11y,
        )
        self.assertIn("title_of: impl Fn(WindowId) -> Option<&'a str>", self.a11y)
        self.assertIn("desc_of: impl Fn(usize) -> Option<&'a str>", self.a11y)
        self.assertIn("recent_desc_of: impl Fn(usize) -> Option<&'a str>", self.a11y)
        self.assertIn("minimized_of: impl Fn(WindowId) -> bool", self.a11y)
        self.assertIn("fn format_tooltip<'a>(", self.a11y)

    def _fn_body(self, src, sig):
        """Brace-balanced body of the function whose signature starts at
        `sig` (a string that appears once in `src`). Shared by the purity and
        delegation pins so both inspect the same slice of source."""
        start = src.index(sig)
        fn_body = src[start:]
        open_brace = fn_body.index("{")
        depth = 0
        for i in range(open_brace, len(fn_body)):
            if fn_body[i] == "{":
                depth += 1
            elif fn_body[i] == "}":
                depth -= 1
                if depth == 0:
                    return fn_body[open_brace : i + 1]
        raise AssertionError("unbalanced braces in " + sig)

    def test_tooltip_label_delegates_to_pure_fn(self):
        # The Desktop adapter must route through format_tooltip so no label
        # string exists outside the pure function. Scoped to the adapter's
        # own body: a refactor that moved the formatting back into
        # tooltip_label (while still calling format_tooltip once, e.g. for an
        # early return) would leave a label literal here and fail the pin.
        body = self._fn_body(self.a11y, "pub(crate) fn tooltip_label(")
        self.assertIn("format_tooltip(", body)
        # No label text may be formatted in the adapter: it supplies only
        # lookups. The `{}` format placeholder would indicate inline
        # formatting; the pure function owns every label literal.
        self.assertNotIn("{}", body)
        # Adapter must supply all four lookups (title/desc/recent/minimized).
        self.assertIn(".title.as_str()", body)
        self.assertIn("app.description", body)
        self.assertIn("WindowState::Minimized", body)

    def test_label_literals_match_selftests(self):
        # These strings are the UI contract the QEMU selftests also assert
        # (test_tooltip_owner_label / test_tooltip_role_labels in
        # ade/src/util/testing/a11y.rs). A rename here must be mirrored there.
        self.assertIn('"{} {}"', self.a11y)
        self.assertIn('WindowButton::Close => "Close"', self.a11y)
        self.assertIn('WindowButton::Minimize => "Minimize"', self.a11y)
        for lit in ('"Switch to {}"', '"Restore {}"', '"Open Start menu"'):
            self.assertIn(lit, self.a11y, "missing literal: " + lit)

    def test_hover_targets_cover_every_interactive_surface(self):
        # Every HoverTarget payload kind that maps to a tooltip must have an
        # arm. StartCategory/StartPower/Tray/ClipboardRow intentionally fall
        # through to the node fallback (they have no label text today).
        self.assertIn("Some(HoverTarget::Window { win, btn })", self.a11y)
        self.assertIn("Some(HoverTarget::TaskbarButton(wid))", self.a11y)
        self.assertIn("Some(HoverTarget::StartApp(i))", self.a11y)
        self.assertIn("Some(HoverTarget::StartRecent(ri))", self.a11y)
        self.assertIn("Some(HoverTarget::StartButton)", self.a11y)


class TooltipFormatPort(unittest.TestCase):
    """Behavior port — same inputs as the QEMU selftests, host-side."""

    def _titles(self, table):
        return lambda wid: table.get(wid)

    def _descs(self, table):
        return lambda i: table.get(i)

    def test_window_close_button(self):
        node = {"role": "Button", "label": "Close", "owner": 7}
        text = format_tooltip(
            node,
            ("Window", 7, "Close"),
            self._titles({7: "SettingsWin"}),
            self._descs({}),
            self._descs({}),
        )
        self.assertEqual(text, "Close SettingsWin")

    def test_window_minimize_button(self):
        node = {"role": "Button", "label": "Minimize", "owner": 7}
        text = format_tooltip(
            node,
            ("Window", 7, "Minimize"),
            self._titles({7: "SettingsWin"}),
            self._descs({}),
            self._descs({}),
        )
        self.assertEqual(text, "Minimize SettingsWin")

    def test_taskbar_button_prefixes_switch_to(self):
        node = {"role": "Button", "label": "TaskbarWin", "owner": 3}
        text = format_tooltip(
            node,
            ("TaskbarButton", 3),
            self._titles({3: "TaskbarWin"}),
            self._descs({}),
            self._descs({}),
        )
        self.assertEqual(text, "Switch to TaskbarWin")

    def test_taskbar_button_minimized_prefixes_restore(self):
        node = {"role": "Button", "label": "MinWin", "owner": 3}
        text = format_tooltip(
            node,
            ("TaskbarButton", 3),
            self._titles({3: "MinWin"}),
            self._descs({}),
            self._descs({}),
            lambda wid: True,
        )
        self.assertEqual(text, "Restore MinWin")

    def test_start_button_names_action(self):
        text = format_tooltip(
            {"role": "Button", "label": "Start", "owner": None},
            ("StartButton",),
            self._titles({}),
            self._descs({}),
            self._descs({}),
        )
        self.assertEqual(text, "Open Start menu")

    def test_start_menu_row_shows_description(self):
        node = {"role": "StartMenu", "label": "", "owner": None}
        text = format_tooltip(
            node,
            ("StartApp", 0),
            self._titles({}),
            self._descs({0: "Shell with pty support"}),
            self._descs({}),
        )
        self.assertEqual(text, "Shell with pty support")

    def test_start_recent_shows_description(self):
        node = {"role": "StartMenu", "label": "", "owner": None}
        text = format_tooltip(
            node,
            ("StartRecent", 1),
            self._titles({}),
            self._descs({}),
            self._descs({1: "Browse and manage files"}),
        )
        self.assertEqual(text, "Browse and manage files")

    def test_unresolvable_lookups_fall_back_to_empty(self):
        # A row index with no app resolves to "" — the caller shows no tooltip.
        node = {"role": "StartMenu", "label": "", "owner": None}
        text = format_tooltip(
            node,
            ("StartApp", 99),
            self._titles({}),
            self._descs({}),
            self._descs({}),
        )
        self.assertEqual(text, "")

    def test_owner_fallback_uses_title(self):
        # No hover target (or a non-tooltip target): owner stamp wins over
        # the node label.
        node = {"role": "Button", "label": "Close", "owner": 7}
        text = format_tooltip(
            node, None, self._titles({7: "MyWin"}), self._descs({}), self._descs({})
        )
        self.assertEqual(text, "MyWin")

    def test_owner_fallback_miss_uses_label(self):
        node = {"role": "Button", "label": "Close", "owner": 7}
        text = format_tooltip(
            node, None, self._titles({}), self._descs({}), self._descs({})
        )
        self.assertEqual(text, "Close")

    def test_plain_node_uses_label(self):
        node = {"role": "Icon", "label": "Terminal", "owner": None}
        text = format_tooltip(
            node, None, self._titles({}), self._descs({}), self._descs({})
        )
        self.assertEqual(text, "Terminal")

    def test_empty_label_uses_role_name(self):
        for role, expected in (("Taskbar", "Taskbar"), ("StartMenu", "Start Menu"), ("Desktop", "Desktop")):
            node = {"role": role, "label": "", "owner": None}
            text = format_tooltip(
                node, None, self._titles({}), self._descs({}), self._descs({})
            )
            self.assertEqual(text, expected)

    def test_empty_label_unknown_role_is_empty(self):
        node = {"role": "Window", "label": "", "owner": None}
        text = format_tooltip(
            node, None, self._titles({}), self._descs({}), self._descs({})
        )
        self.assertEqual(text, "")

    def test_tray_and_clipboard_fall_through_to_node(self):
        # Tray/ClipboardRow have no dedicated arm: they reach the `_` arm and
        # show the node fallback (owner title or label).
        node = {"role": "Button", "label": "TrayIcon", "owner": None}
        text = format_tooltip(
            node, ("Tray", 0), self._titles({}), self._descs({}), self._descs({})
        )
        self.assertEqual(text, "TrayIcon")


if __name__ == "__main__":
    unittest.main()
