//! Keymap routing tests — byte decode, modifier state, routing table.
//!
//! Pins the Phase 3 input extraction: `KeyEvent::from_byte` must decode the
//! producer's byte stream exactly as the legacy magic constants did, and the
//! routing table must resolve the same actions and the same terminal grabs
//! as the old `ShortcutManager` bindings + `DESKTOP_KEYS` list.

use crate::core::desktop::Desktop;
use crate::core::event::Event;
use crate::core::window::{AppWindow, WindowState};
use crate::input::keys;
use crate::input::{dump_bindings, is_desktop_shortcut, resolve, KeyAction, KeyEvent};
use libsarga::io;

pub(crate) fn test_keymap() -> bool {
    // The producer folds Ctrl+letter into ASCII control codes.
    let ev = KeyEvent::from_byte(23); // Ctrl+W
    if ev.code != b'w' || !ev.ctrl {
        io::print_str("[test] FAIL test_keymap: ctrl+w decode\n");
        return false;
    }
    if resolve(ev) != Some(KeyAction::CloseFocused) {
        io::print_str("[test] FAIL test_keymap: ctrl+w -> CloseFocused\n");
        return false;
    }
    // Every legacy shortcut binding resolves identically (Ctrl+Shift+S /
    // Ctrl+Shift+X arrive as plain Ctrl+S / Ctrl+X — undecodable from a
    // single byte, same as before). Ctrl+Q is NOT here: it is deliberately
    // unbound since the session-end gate moved to the Ctrl+Alt+Backspace
    // chord (Phase C). Esc-on-empty-desktop is the second session-end path
    // but a contextual a11y-arm behavior, not a table binding.
    let legacy: [(u8, KeyAction); 10] = [
        (1, KeyAction::ToggleAot),
        (2, KeyAction::ClipboardPanel),
        (3, KeyAction::ClearNotifications),
        (4, KeyAction::DismissNotification),
        (5, KeyAction::CycleWindow),
        (14, KeyAction::DemoNotification),
        (19, KeyAction::OpenSettings),
        (20, KeyAction::CycleTiling),
        (23, KeyAction::CloseFocused),
        (24, KeyAction::OpenTaskManager),
    ];
    for (byte, expected) in legacy {
        if resolve(KeyEvent::from_byte(byte)) != Some(expected) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_keymap: legacy binding byte {byte}\n"
            ));
            return false;
        }
    }
    // Ctrl+Q must be unbound — the session-end gate is the chord now.
    if resolve(KeyEvent::from_byte(17)).is_some() {
        io::print_str("[test] FAIL test_keymap: ctrl+q still resolves to an action\n");
        return false;
    }
    // Terminal routing: exactly the old DESKTOP_KEYS stay desktop-side
    // (now nine — Ctrl+Q dropped with its binding).
    const DESKTOP_KEYS: [u8; 9] = [1, 2, 4, 5, 14, 19, 20, 23, 24];
    for byte in 0..=26u8 {
        let ev = KeyEvent::from_byte(byte);
        if is_desktop_shortcut(ev) != DESKTOP_KEYS.contains(&byte) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_keymap: desktop grab byte {byte}\n"
            ));
            return false;
        }
    }
    // Ctrl+C (interrupt) and Ctrl+L (shell clear) stay with the terminal.
    if is_desktop_shortcut(KeyEvent::from_byte(3)) || is_desktop_shortcut(KeyEvent::from_byte(12)) {
        io::print_str("[test] FAIL test_keymap: ctrl+c/ctrl+l must reach the shell\n");
        return false;
    }
    // The Ctrl+Alt+Backspace logout chord is a desktop grab (so it works
    // from a terminal) and resolves to Quit. Partial-modifier Backspace
    // (ctrl-only / alt-only) must NOT be grabbed — plain Backspace edits.
    let chord = KeyEvent::new(keys::KEY_BACKSPACE, true, true, false);
    if resolve(chord) != Some(KeyAction::Quit) || !is_desktop_shortcut(chord) {
        io::print_str("[test] FAIL test_keymap: ctrl+alt+backspace != Quit grab\n");
        return false;
    }
    if is_desktop_shortcut(KeyEvent::new(keys::KEY_BACKSPACE, true, false, false))
        || is_desktop_shortcut(KeyEvent::new(keys::KEY_BACKSPACE, false, true, false))
    {
        io::print_str("[test] FAIL test_keymap: partial-modifier backspace grabbed\n");
        return false;
    }
    // Backspace, Escape, and plain 'q' also stay with the terminal (they are
    // not desktop grabs; Escape is a11y-consumed before handle_key anyway).
    for byte in [keys::KEY_BACKSPACE, keys::KEY_ESC, keys::KEY_Q] {
        if is_desktop_shortcut(KeyEvent::from_byte(byte)) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_keymap: byte {byte} must reach the shell\n"
            ));
            return false;
        }
    }
    // Enter/backspace normalization across ASCII and scan codes.
    if KeyEvent::from_byte(keys::KEY_LF).code != keys::KEY_ENTER
        || KeyEvent::from_byte(keys::SCAN_ENTER).code != keys::KEY_ENTER
        || KeyEvent::from_byte(keys::KEY_BACKSPACE_ALT).code != keys::KEY_BACKSPACE
    {
        io::print_str("[test] FAIL test_keymap: enter/backspace normalization\n");
        return false;
    }
    // Plain letters are text, not shortcuts; Ctrl+letter never types text.
    let a = KeyEvent::from_byte(b'a');
    if a.text() != Some('a') || resolve(a).is_some() {
        io::print_str("[test] FAIL test_keymap: plain text\n");
        return false;
    }
    if KeyEvent::from_byte(17).text().is_some() {
        io::print_str("[test] FAIL test_keymap: ctrl keys are not text\n");
        return false;
    }
    // Arrows arrive from the producer as plain set-1 scan bytes (the
    // RawKey forward set spec'd in kernel-keyboard-gate.md §2.1 — the E0
    // second byte for the arrows). `from_byte` must round-trip them
    // untouched (72/75/77/80 collide with none of its special cases) and
    // they must be UNBOUND and UNGRABBED: the a11y pre-handler is their
    // only consumer, so a future keymap row would be dead weight (the a11y
    // arm consumes plain arrows before `handle_key` runs) and a
    // `desktop: true` row would needlessly steal them from a terminal.
    // NOTE the byte-stream collision: 72='H', 75='K', 77='M', 80='P' — the
    // plain arrow bytes decode as TEXT, which is exactly why the a11y arm
    // must consume them before the typing path can. The shifted arrow
    // (once the kernel packs mods — Design A, bit10 = shift) must be fully
    // inert: no binding, no grab, no text (`text()` rejects shift).
    for code in [
        keys::SCAN_UP,
        keys::SCAN_LEFT,
        keys::SCAN_RIGHT,
        keys::SCAN_DOWN,
    ] {
        let ev = KeyEvent::from_byte(code);
        if ev.code != code || ev.ctrl || ev.alt || ev.shift {
            io::print_str(&alloc::format!(
                "[test] FAIL test_keymap: arrow byte {code} mangled by from_byte\n"
            ));
            return false;
        }
        if resolve(ev).is_some() || is_desktop_shortcut(ev) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_keymap: arrow {code} is bound/grabbed — the a11y pre-handler must be its only consumer\n"
            ));
            return false;
        }
        if ev.text().is_none() {
            io::print_str(&alloc::format!(
                "[test] FAIL test_keymap: arrow byte {code} lost its ASCII collision — the a11y arm is no longer load-bearing against the typing path\n"
            ));
            return false;
        }
        let shifted = KeyEvent::from_raw(code as u16 | (1 << 10));
        if !shifted.shift || shifted.code != code {
            io::print_str(&alloc::format!(
                "[test] FAIL test_keymap: shifted arrow {code} decode (bit10 = shift)\n"
            ));
            return false;
        }
        if resolve(shifted).is_some() || is_desktop_shortcut(shifted) || shifted.text().is_some() {
            io::print_str(&alloc::format!(
                "[test] FAIL test_keymap: shifted arrow {code} must be fully inert — no binding, no grab, no text\n"
            ));
            return false;
        }
    }
    // Host-verifiable table contract: the `resolve`-derived dump pins the
    // binding count and the session-end invariants. A keymap edit that
    // accidentally re-adds a session-end binding (e.g. Ctrl+Q -> Quit) or
    // grows/shrinks the table fails HERE — first in the suite, before any
    // QEMU run. The count is a deliberate tripwire: update it knowingly
    // when the table legitimately changes. (test_session_end_gate overlaps
    // the chord/Ctrl+Q resolve checks at the Desktop level; this is the
    // table-level pin and runs first — don't collapse the two.)
    let dump = dump_bindings();
    if dump.count != 18 {
        io::print_str(&alloc::format!(
            "[test] FAIL test_keymap: binding count {} != 18 (update pin deliberately)\n",
            dump.count
        ));
        return false;
    }
    if dump.quit_count != 1 {
        io::print_str(&alloc::format!(
            "[test] FAIL test_keymap: {} quit bindings != 1 (only ctrl+alt+backspace is a Quit binding; esc-on-empty-desktop is contextual)\n",
            dump.quit_count
        ));
        return false;
    }
    if !dump.has_quit_chord {
        io::print_str("[test] FAIL test_keymap: ctrl+alt+backspace chord missing from table\n");
        return false;
    }
    if !dump.ctrl_q_unbound {
        io::print_str(
            "[test] FAIL test_keymap: ctrl+q re-bound — the chord is the only Quit binding; esc-on-empty-desktop is contextual\n",
        );
        return false;
    }
    // Desktop-grab tuple count: the number of distinct events that
    // is_desktop_shortcut grabs over the whole event space. Today that is
    // the 9 Ctrl+letter grabs + the Ctrl+Alt+Backspace chord = 10. This is
    // the tripwire for a table edit that turns a terminal key into a
    // desktop grab: Ctrl+C/Ctrl+L must stay with the shell, so any growth
    // here (or a drop, which would silently stop grabbing a shortcut)
    // fails the selftest with a host-verifiable signal.
    if dump.desktop_grabs != 10 {
        io::print_str(&alloc::format!(
            "[test] FAIL test_keymap: desktop-grab tuple count {} != 10 (9 ctrl+letter grabs + ctrl+alt+backspace chord; update pin deliberately)\n",
            dump.desktop_grabs
        ));
        return false;
    }
    // Byte-deliverability round trip: every binding tuple in the table must
    // be reachable from the producer's single-byte stream via `from_byte`,
    // or be a documented synthetic-only chord. `from_byte` recovers only
    // Ctrl — alt/shift always decode false — so a future Alt/Shift binding
    // (e.g. Alt+Tab) would be unreachable from real hardware until the
    // kernel delivers modifier bits (the Phase C gate); this pin makes that
    // failure explicit instead of adding a row the stream can never fire.
    // Verified against the source (simulated `from_byte` over the 18-row
    // table): exactly 17 rows have a delivering byte; the Ctrl+Alt+Backspace
    // Quit chord is the sole synthetic-only row, constructed with
    // `KeyEvent::new` and asserted below. A table edit that adds an
    // undeliverable binding, or forgets that a new Alt/Shift row needs a
    // documented synthetic producer, fails here.
    let mut deliverable = 0u32;
    let mut synthetic = 0u32;
    let mut synthetic_is_chord = true;
    for b in crate::input::BINDINGS {
        if b.alt || b.shift {
            synthetic += 1;
            synthetic_is_chord &= b.code == keys::KEY_BACKSPACE && b.ctrl && b.alt && !b.shift;
            continue;
        }
        let mut delivered = false;
        for x in 0..=255u8 {
            if KeyEvent::from_byte(x) == KeyEvent::new(b.code, b.ctrl, false, false) {
                delivered = true;
                break;
            }
        }
        if !delivered {
            io::print_str(&alloc::format!(
                "[test] FAIL test_keymap: binding (code {}, ctrl {}) has no from_byte input — the byte stream can never trigger it; a binding the producer cannot deliver is dead weight or needs a synthetic producer\n",
                b.code, b.ctrl
            ));
            return false;
        }
        deliverable += 1;
    }
    if deliverable != 17 {
        io::print_str(&alloc::format!(
            "[test] FAIL test_keymap: {} byte-deliverable bindings != 17 (every row except the ctrl+alt+backspace chord; update pin deliberately)\n",
            deliverable
        ));
        return false;
    }
    if synthetic != 1 || !synthetic_is_chord {
        io::print_str("[test] FAIL test_keymap: synthetic-only rows != exactly the ctrl+alt+backspace chord (from_byte never sets alt/shift — a new one cannot arrive via the byte stream until the kernel sends modifier bits)\n");
        return false;
    }
    // Packed-stream deliverability: once the kernel sends modifier bits
    // (Phase C, Design A — `KeyEvent::from_raw`), EVERY binding row must be
    // reachable from a synthetic (byte, mods) pair — nothing may stay
    // synthetic. The byte stream leaves the chord synthetic (pinned above);
    // the packed stream delivers it as 0x0308, so the whole 18-row table
    // becomes realizable from real input the day the kernel lands. A future
    // Alt/Shift binding (e.g. Alt+Tab) must be reachable HERE even though
    // the byte stream can't fire it — this sweep is the forward contract.
    let mut packed_unreachable = 0u32;
    let mut first_packed_code = 0u8;
    let mut first_packed_ctrl = false;
    for b in crate::input::BINDINGS {
        let target = KeyEvent::new(b.code, b.ctrl, b.alt, b.shift);
        let mut delivered_raw: Option<u16> = None;
        'search: for byte in 0..=255u8 {
            for alt in [false, true] {
                for ctrl in [false, true] {
                    for shift in [false, true] {
                        let mut raw = byte as u16;
                        if alt {
                            raw |= 1 << 8;
                        }
                        if ctrl {
                            raw |= 1 << 9;
                        }
                        if shift {
                            raw |= 1 << 10;
                        }
                        if KeyEvent::from_raw(raw) == target {
                            delivered_raw = Some(raw);
                            break 'search;
                        }
                    }
                }
            }
        }
        match delivered_raw {
            Some(raw) => {
                // Decode THROUGH resolve: the (byte, mods) pair must map to
                // this binding's action, not just to its tuple — a duplicate
                // or shadowing row (resolve returns the first match) fails
                // here even though the tuple-equality check above passes.
                if resolve(KeyEvent::from_raw(raw)) != Some(b.action) {
                    io::print_str(&alloc::format!(
                        "[test] FAIL test_keymap: packed pair 0x{:04x} decodes to binding (code {}, ctrl {}) but resolve maps it to a different action — shadowing or duplicate row?\n",
                        raw, b.code, b.ctrl
                    ));
                    return false;
                }
            }
            None => {
                packed_unreachable += 1;
                first_packed_code = b.code;
                first_packed_ctrl = b.ctrl;
            }
        }
    }
    if packed_unreachable != 0 {
        io::print_str(&alloc::format!(
            "[test] FAIL test_keymap: {} binding(s) unreachable via from_raw (first: code {}, ctrl {}) — the packed stream must deliver every row once the kernel sends modifier bits; a binding no (byte, mods) pair can produce is dead weight\n",
            packed_unreachable, first_packed_code, first_packed_ctrl
        ));
        return false;
    }
    io::print_str("[test] PASS test_keymap\n");
    true
}

/// Phase C packed-key decode (Design A — docs/kernel-gui-modifier-delivery.md).
/// Pins `KeyEvent::from_raw`, the userspace half of the kernel modifier
/// contract: low byte = char, bit8 = alt, bit9 = ctrl, bit10 = shift,
/// bit11 (super) ignored. When the high bits are zero it must be
/// byte-identical to `from_byte` (inert until the kernel sends bits), and the
/// packed chord 0x0308 (`0x08 | alt<<8 | ctrl<<9`) must decode to the exact
/// tuple the routing table's Quit row matches — so a kernel rewrite that
/// swaps the ctrl/alt bit order fails here, not in QEMU.
pub(crate) fn test_from_raw() -> bool {
    // Inertness: zero high bits ⇒ identical to the byte decode. This is the
    // property that lets the u16 path land before the kernel sends bits.
    for b in 0..=255u8 {
        if KeyEvent::from_raw(b as u16) != KeyEvent::from_byte(b) {
            io::print_str(&alloc::format!(
                "[test] FAIL test_from_raw: from_raw({}) != from_byte({}) — the packed decode must be additive over the byte decode\n",
                b, b
            ));
            return false;
        }
    }
    // Spec table (Design A bit layout).
    if KeyEvent::from_raw(0x63) != KeyEvent::new(b'c', false, false, false) {
        io::print_str("[test] FAIL test_from_raw: plain 0x63\n");
        return false;
    }
    if KeyEvent::from_raw(0x0D) != KeyEvent::new(keys::KEY_ENTER, false, false, false) {
        io::print_str("[test] FAIL test_from_raw: plain 0x0D\n");
        return false;
    }
    // The chord: 0x08 | (1<<8) | (1<<9) = 0x0308 → Backspace + alt + ctrl.
    let chord = KeyEvent::from_raw(0x0308);
    if chord != KeyEvent::new(keys::KEY_BACKSPACE, true, true, false) {
        io::print_str("[test] FAIL test_from_raw: chord 0x0308 decode\n");
        return false;
    }
    if resolve(chord) != Some(KeyAction::Quit) {
        io::print_str("[test] FAIL test_from_raw: chord 0x0308 -> Quit\n");
        return false;
    }
    if !is_desktop_shortcut(chord) {
        io::print_str("[test] FAIL test_from_raw: chord 0x0308 must be a desktop grab\n");
        return false;
    }
    // Partial-modifier decodes pin the bit order (alt=8, ctrl=9).
    if KeyEvent::from_raw(0x0108) != KeyEvent::new(keys::KEY_BACKSPACE, false, true, false) {
        io::print_str("[test] FAIL test_from_raw: bit8 must be alt (0x0108)\n");
        return false;
    }
    if KeyEvent::from_raw(0x0208) != KeyEvent::new(keys::KEY_BACKSPACE, true, false, false) {
        io::print_str("[test] FAIL test_from_raw: bit9 must be ctrl (0x0208)\n");
        return false;
    }
    // Bit 11 (Super) is ignored by ade; Ctrl+C low-byte fold still works
    // when an alt bit rides along.
    if KeyEvent::from_raw(0x0808) != KeyEvent::from_raw(0x0008) {
        io::print_str("[test] FAIL test_from_raw: super bit (11) must be ignored\n");
        return false;
    }
    if KeyEvent::from_raw(0x0103) != KeyEvent::new(b'c', true, true, false) {
        io::print_str("[test] FAIL test_from_raw: ctrl+C + alt bit (0x0103)\n");
        return false;
    }
    io::print_str("[test] PASS test_from_raw\n");
    true
}

/// Session-end gate: two session-end paths, both only with an empty window
/// list. (1) The Ctrl+Alt+Backspace chord — a keymap grab, synthetic-only
/// because the byte stream cannot deliver Alt yet (the Phase C kernel gate).
/// (2) Esc on an empty desktop — 0x1B is the one distinct control byte the
/// stream does carry, so a hardware Esc reaches userspace today; it ends the
/// session only when nothing is open (no a11y ring, fullscreen window,
/// windows, switcher, or overlay). Esc is otherwise the single dismiss key:
/// it closes every overlay, exits fullscreen, and only then falls through to
/// the empty-desktop logout — all in the a11y Esc arm, since that arm
/// consumes Esc before `handle_key` (and its keymap grab) ever runs. Ctrl+Q
/// and plain 'q' are deliberately unbound (the old gates are gone), and
/// Backspace is never a session key — it edits text in plain windows and
/// reaches the shell inside a terminal — so typing can never trip the logout
/// loop.
///
/// A third leg pins ROUTING, not session end: with a pty window focused,
/// the packed chord must bypass the terminal-forward path (it is a desktop
/// grab — `is_desktop_shortcut`), so the day the kernel delivers 0x0308 the
/// packed stream can't be eaten by terminal routing. Observed through a real
/// openpty slave read: the chord byte must never reach the pty master, while
/// a plain key must (proving the observer and the forward path both work).
///
/// Each sub-case runs on its own fresh `Desktop` so an ending session can't
/// leak into the shared desktop passed through `run_all`.
pub(crate) fn test_session_end_gate() -> bool {
    // Ctrl+Alt+Backspace resolves to Quit; Ctrl+Q (0x11) must resolve to
    // nothing — the chord replaced it as the session-end key.
    let chord = KeyEvent::new(keys::KEY_BACKSPACE, true, true, false);
    if resolve(chord) != Some(KeyAction::Quit) {
        io::print_str("[test] FAIL test_session_end_gate: ctrl+alt+backspace != Quit\n");
        return false;
    }
    if resolve(KeyEvent::from_byte(17)).is_some() {
        io::print_str("[test] FAIL test_session_end_gate: ctrl+q still resolves\n");
        return false;
    }

    // Backspace on an empty desktop must not end the session (nothing
    // focused, so it is a silent no-op).
    let mut d = Desktop::new(800, 600);
    d.handle_event(Event::Key(keys::KEY_BACKSPACE as u16));
    if d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: backspace ended empty session\n");
        return false;
    }

    // Backspace in a focused plain (non-terminal) window must EDIT, not end
    // the session — this was the Gap 3 footgun (the old main.rs gate killed
    // the session on any Backspace outside a pty, so plain windows could
    // never edit text). `wm.create` leaves the window focused.
    let mut d = Desktop::new(800, 600);
    let win = AppWindow::new(100, 100, 400, 300, "Editor");
    d.wm.create(win);
    d.handle_event(Event::Key(b'a' as u16)); // type a char into the surface
    d.handle_event(Event::Key(keys::KEY_BACKSPACE as u16));
    if d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: backspace ended session in window\n");
        return false;
    }
    let focused = d.wm.focused_mut().unwrap();
    if focused.surface().last_line().is_some_and(|l| !l.is_empty()) {
        io::print_str("[test] FAIL test_session_end_gate: backspace did not edit the window\n");
        return false;
    }

    // The old gates are gone: Ctrl+Q and plain 'q' never end a session,
    // even on an empty desktop. QEMU-facing counterpart:
    // qemu_gui_login.exp step 4b sends a REAL Ctrl+Q (monitor sendkey) on
    // the empty desktop and asserts the session does not end or respawn.
    // The kernel decodes with pc-keyboard 0.5.1 HandleControl::Ignore, so
    // the key arrives as plain 'q' (0x71) — equally unbound — meaning BOTH
    // arms here are the synthetic pins for that single real-input probe.
    let mut d = Desktop::new(800, 600);
    d.handle_event(Event::Key(17)); // Ctrl+Q
    if d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: ctrl+q ended empty session\n");
        return false;
    }
    let mut d = Desktop::new(800, 600);
    d.handle_event(Event::Key(keys::KEY_Q as u16));
    if d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: plain q ended empty session\n");
        return false;
    }

    // The chord with a window open is a deliberate no-op (collision-proof:
    // Ctrl+Alt+Backspace can't kill a session mid-work).
    let mut d = Desktop::new(800, 600);
    d.wm.create(AppWindow::new(100, 100, 400, 300, "GateWin"));
    d.handle_key_event(chord);
    if d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: chord ended session with window open\n");
        return false;
    }

    // The chord on an empty desktop ends the session.
    let mut d = Desktop::new(800, 600);
    d.handle_key_event(chord);
    if !d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: chord did not end empty session\n");
        return false;
    }

    // The chord driven through the REAL event path — Event::Key(u16) ->
    // handle_a11y_key (modifier guard) -> handle_key -> from_raw -> resolve ->
    // Quit. This is exactly what the kernel will deliver as 0x0308 once the
    // modifier bits land (Phase C, Design A); the synthetic
    // `handle_key_event` cases above bypass the a11y pre-handler and the
    // decode, so this case pins the observable end-to-end behavior. Note:
    // Backspace (0x08) has no a11y match arm today, so a removed guard would
    // still fall through `_ => false` and not fail THIS test — the guard is
    // load-bearing only via the host source pin
    // (TestKernelKeyContract::test_desktop_routes_via_from_raw_and_guards_a11y).
    let mut d = Desktop::new(800, 600);
    d.handle_event(Event::Key(0x0308));
    if !d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: packed chord 0x0308 did not end empty session (a11y guard or decode ate it)\n");
        return false;
    }
    let mut d = Desktop::new(800, 600);
    d.wm.create(AppWindow::new(100, 100, 400, 300, "GateWin2"));
    d.handle_event(Event::Key(0x0308));
    if d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: packed chord 0x0308 ended session with window open\n");
        return false;
    }
    // Partial modifiers must NOT end the session: 0x0108 (alt-only) and
    // 0x0208 (ctrl-only) decode to Backspace with one bit, matching neither
    // the chord row (needs both) nor the plain Backspace row (needs none) —
    // a bit-order swap in the kernel or the decode surfaces here, not in
    // QEMU.
    let mut d = Desktop::new(800, 600);
    d.handle_event(Event::Key(0x0108));
    if d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: alt-only backspace (0x0108) ended empty session — bit order wrong?\n");
        return false;
    }
    let mut d = Desktop::new(800, 600);
    d.handle_event(Event::Key(0x0208));
    if d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: ctrl-only backspace (0x0208) ended empty session — bit order wrong?\n");
        return false;
    }

    // Terminal-focused real-path chord: with a pty window focused, the
    // packed chord 0x0308 must NOT be eaten by the terminal-forward path
    // (`terminal_focused && !is_desktop_shortcut` only forwards keys that
    // are NOT desktop grabs; the chord IS a grab, so it bypasses the pty
    // write and reaches the keymap router). With the terminal open the
    // session cannot end (the Quit arm requires an empty window list — the
    // collision-proof no-op pinned above), so the observable contract is:
    // the byte NEVER reaches the pty master — observed through the slave
    // read end of a real openpty (the kernel pty queues master writes into
    // the slave buffer synchronously; a read returns queued bytes without
    // blocking). A control case drives a plain 'a' through the same
    // terminal and asserts the byte DID reach the pty — proving the
    // observer works and non-grab keys still forward, so the chord's empty
    // slave is the grab bypass, not a dead write path. (Synthetic
    // `handle_key_event` can't pin this: the chord never reaches the pty
    // write via either route, but only the real `Event::Key` path exercises
    // the `from_raw` decode + grab check together.)
    let (master, slave) = match libsarga::io::openpty() {
        Ok(p) => p,
        Err(_) => {
            io::print_str("[test] FAIL test_session_end_gate: openpty failed\n");
            return false;
        }
    };
    let mut d = Desktop::new(800, 600);
    let wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "TermWin"));
    d.wm.lookup_mut(wid).unwrap().attach_terminal(master);
    if !d.focused_has_pty() {
        io::print_str("[test] FAIL test_session_end_gate: terminal window not focused\n");
        return false;
    }
    d.handle_event(Event::Key(0x0308));
    if d.session.is_ending() {
        io::print_str(
            "[test] FAIL test_session_end_gate: chord ended session with terminal open\n",
        );
        return false;
    }
    let mut buf = [0u8; 8];
    let n = match libsarga::io::read(slave, &mut buf) {
        Ok(n) => n,
        Err(_) => {
            io::print_str("[test] FAIL test_session_end_gate: slave read failed\n");
            return false;
        }
    };
    if n != 0 {
        io::print_str(&alloc::format!(
            "[test] FAIL test_session_end_gate: chord byte 0x{:02x} reached the pty — terminal routing ate the desktop grab\n",
            buf[0]
        ));
        return false;
    }
    // Control: a non-grab key MUST be forwarded to the pty — the forward
    // path works and the observer sees it, so the chord's empty slave above
    // is the grab bypass, not a dead write.
    d.handle_event(Event::Key(b'a' as u16));
    let n = libsarga::io::read(slave, &mut buf).unwrap_or_default();
    if n != 1 || buf[0] != b'a' {
        io::print_str(
            "[test] FAIL test_session_end_gate: plain key did not reach the pty — observer broken or forward path dead\n",
        );
        return false;
    }
    let _ = libsarga::io::close(slave);
    let _ = libsarga::io::close(master);

    // Esc-on-empty-desktop is the byte-deliverable session-end path: 0x1B is
    // the one distinct control byte the key stream carries, so a hardware Esc
    // reaches userspace today (the chord needs Alt, which the kernel does not
    // deliver yet). Driven through the real `handle_event` byte path.
    let mut d = Desktop::new(800, 600);
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if !d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: esc did not end empty session\n");
        return false;
    }

    // Esc with a window open must NOT end the session — Esc dismisses, it is
    // not a logout key while anything is open.
    let mut d = Desktop::new(800, 600);
    d.wm.create(AppWindow::new(100, 100, 400, 300, "EscWin"));
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: esc ended session with window open\n");
        return false;
    }

    // Esc on a fullscreen window EXITS fullscreen — the behavior that lived
    // in the keymap `KeyAction::Escape` grab, consolidated into the a11y Esc
    // arm because that grab was unreachable from the real event path (this
    // arm consumes Esc before `handle_key` ever runs). A real hardware Esc
    // reaches the exit; the session must NOT end (a fullscreen window is
    // still a window).
    let mut d = Desktop::new(800, 600);
    let wid = d.wm.create(AppWindow::new(100, 100, 400, 300, "FullWin"));
    d.wm.toggle_fullscreen(wid, d.screen_w, d.screen_h);
    if !d
        .wm
        .lookup(wid)
        .is_some_and(|w| w.state == WindowState::Fullscreen)
    {
        io::print_str("[test] FAIL test_session_end_gate: window did not enter fullscreen\n");
        return false;
    }
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if d.session.is_ending() {
        io::print_str(
            "[test] FAIL test_session_end_gate: esc ended session from fullscreen window\n",
        );
        return false;
    }
    if !d
        .wm
        .lookup(wid)
        .is_some_and(|w| w.state == WindowState::Normal)
    {
        io::print_str("[test] FAIL test_session_end_gate: esc did not exit fullscreen\n");
        return false;
    }

    // Esc with the start menu open closes the menu and must NOT end the
    // session (the dismissal is the empty-desktop check's precondition).
    let mut d = Desktop::new(800, 600);
    d.start_menu.open = true;
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if d.session.is_ending() || d.start_menu.open {
        io::print_str(
            "[test] FAIL test_session_end_gate: esc with menu open ended session or kept menu\n",
        );
        return false;
    }

    // Esc with a modal panel open (settings app) must NOT end the session —
    // and must actually CLOSE the panel (the dismiss half of the a11y arm;
    // the old keymap grab could never fire, so real Esc closing these
    // overlays is the consolidated contract).
    let mut d = Desktop::new(800, 600);
    d.settings_app.open = true;
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if d.session.is_ending() || d.settings_app.open {
        io::print_str(
            "[test] FAIL test_session_end_gate: esc with settings app open ended session or kept it\n",
        );
        return false;
    }

    // The remaining overlays in the `nothing_open` predicate are pinned too,
    // so a future edit that drops one from `overlay_open()` fails here
    // instead of silently ending sessions with that panel open.
    let mut d = Desktop::new(800, 600);
    d.settings.open = true;
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if d.session.is_ending() || d.settings.open {
        io::print_str(
            "[test] FAIL test_session_end_gate: esc with settings panel open ended session or kept it\n",
        );
        return false;
    }
    let mut d = Desktop::new(800, 600);
    d.task_manager.open = true;
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if d.session.is_ending() || d.task_manager.open {
        io::print_str(
            "[test] FAIL test_session_end_gate: esc with task manager open ended session or kept it\n",
        );
        return false;
    }
    let mut d = Desktop::new(800, 600);
    d.about_state.open = true;
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if d.session.is_ending() || d.about_state.open {
        io::print_str(
            "[test] FAIL test_session_end_gate: esc with about open ended session or kept it\n",
        );
        return false;
    }
    // Esc mid-drag must NOT end the session — a drag is activity, not empty.
    let mut d = Desktop::new(800, 600);
    d.drag_active = true;
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: esc ended session mid-drag\n");
        return false;
    }

    // Esc with the a11y focus ring active dismisses the ring first; the next
    // Esc — now on a truly empty desktop — is the one that ends the session.
    let mut d = Desktop::new(800, 600);
    d.focus_visible = true;
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if d.session.is_ending() || d.focus_visible {
        io::print_str(
            "[test] FAIL test_session_end_gate: esc with ring active ended session or kept ring\n",
        );
        return false;
    }
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if !d.session.is_ending() {
        io::print_str("[test] FAIL test_session_end_gate: second esc did not end empty session\n");
        return false;
    }

    // Near-miss chords must not end the session: exact match on code AND
    // all three modifier bits. Ctrl+Backspace, Alt+Backspace,
    // Ctrl+Alt+Shift+Backspace, and Ctrl+Alt+Q are all no-ops.
    let near_misses = [
        KeyEvent::new(keys::KEY_BACKSPACE, true, false, false),
        KeyEvent::new(keys::KEY_BACKSPACE, false, true, false),
        KeyEvent::new(keys::KEY_BACKSPACE, true, true, true),
        KeyEvent::new(keys::KEY_Q, true, true, false),
    ];
    for ev in near_misses {
        let mut d = Desktop::new(800, 600);
        d.handle_key_event(ev);
        if d.session.is_ending() {
            io::print_str("[test] FAIL test_session_end_gate: near-miss chord ended session\n");
            return false;
        }
    }

    io::print_str("[test] PASS test_session_end_gate\n");
    true
}

/// Full logout protocol from the injected chord — the loop-closing test
/// between the keymap gate and the init respawn contract, without QEMU:
///
///   inject Ctrl+Alt+Backspace on an empty desktop
///     -> `Desktop::handle_key_event` routes to `KeyAction::Quit`
///     -> `SessionManager::request_end()` flips `is_ending()`
///     -> `exit_code()` is the clean 0 that `main.rs` hands to `init`
///        (init resets its crash counter on 0 and respawns login-manager,
///        so MAX_RESPAWNS is never touched by a deliberate logout).
///
/// This is the glue the other tests cover piecewise: `test_keymap` pins the
/// chord in the routing table, `test_session_end_gate` pins the Desktop-side
/// no-op rules, and `test_session_end_protocol` pins `SessionManager` in
/// isolation — this one drives the whole chain from a single injected key.
/// The second half of the test re-runs the identical protocol with a real
/// hardware Esc (0x1B, the one distinct control byte the key stream carries)
/// through `handle_event`'s byte path, proving the byte-deliverable
/// session-end path has the same contract as the synthetic chord.
/// Coverage boundary: it asserts `exit_code()`, not that `main.rs`'s
/// `while !is_ending()` loop actually unwinds and returns it — that binary
/// loop is the QEMU harnesses' `[ade] session ended` grep.
pub(crate) fn test_logout_protocol_from_chord() -> bool {
    let chord = KeyEvent::new(keys::KEY_BACKSPACE, true, true, false);

    // Precondition: a fresh desktop is running with nothing focused.
    let mut d = Desktop::new(800, 600);
    if d.session.is_ending() {
        io::print_str(
            "[test] FAIL test_logout_protocol_from_chord: fresh desktop already ending\n",
        );
        return false;
    }
    if d.session.exit_code() != 0 {
        io::print_str(
            "[test] FAIL test_logout_protocol_from_chord: fresh desktop exit code != 0\n",
        );
        return false;
    }

    // The injected chord is the ONLY logout trigger: keymap resolve ->
    // Desktop routing (Quit arm requires an empty window list) -> session.
    d.handle_key_event(chord);
    if !d.session.is_ending() {
        io::print_str(
            "[test] FAIL test_logout_protocol_from_chord: chord did not flip is_ending\n",
        );
        return false;
    }

    // The exit code main.rs returns to init: EXIT_LOGOUT = 0, a clean exit
    // that init's crash accounting treats as graceful (respawn, no count).
    if d.session.exit_code() != 0 {
        io::print_str("[test] FAIL test_logout_protocol_from_chord: logout exit code != 0\n");
        return false;
    }

    // Idempotent: a second chord (or the main loop re-reading is_ending)
    // keeps the same stable exit code.
    d.handle_key_event(chord);
    if !d.session.is_ending() || d.session.exit_code() != 0 {
        io::print_str(
            "[test] FAIL test_logout_protocol_from_chord: re-inject changed ending/exit\n",
        );
        return false;
    }

    // Near-miss sweep: with the session already ending, keys that could
    // plausibly corrupt the unwind must be inert. Ctrl+Q (byte 0x11, the
    // historical session-end gate) and plain Backspace (byte 0x08, the
    // legacy terminate key) must NOT flip is_ending or exit_code — the
    // unwind is driven only by is_ending() in the main loop, and
    // request_end() is idempotent, so neither may disturb the sequence.
    let ctrl_q = KeyEvent::from_byte(0x11); // folds to Ctrl+Q
    let plain_bsp = KeyEvent::from_byte(keys::KEY_BACKSPACE_ALT);
    d.handle_key_event(ctrl_q);
    d.handle_key_event(plain_bsp);
    if !d.session.is_ending() || d.session.exit_code() != 0 {
        io::print_str(
            "[test] FAIL test_logout_protocol_from_chord: near-miss key corrupted the logout\n",
        );
        return false;
    }

    // — Esc twin — the byte-deliverable session-end path. 0x1B is the one
    // distinct control byte the key stream actually carries, so a hardware
    // Esc on an empty desktop reaches userspace today (the chord needs Alt,
    // which the kernel does not deliver yet). Driven through the real
    // `handle_event` path: `handle_a11y_key` consumes Esc -> dismisses the
    // ring/overlays (none here) -> empty desktop -> `request_end()`. The
    // full protocol must match the chord exactly.
    let mut d = Desktop::new(800, 600);
    if d.session.is_ending() {
        io::print_str(
            "[test] FAIL test_logout_protocol_from_chord: esc twin fresh desktop already ending\n",
        );
        return false;
    }
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if !d.session.is_ending() {
        io::print_str(
            "[test] FAIL test_logout_protocol_from_chord: esc twin did not flip is_ending\n",
        );
        return false;
    }
    if d.session.exit_code() != 0 {
        io::print_str("[test] FAIL test_logout_protocol_from_chord: esc twin exit code != 0\n");
        return false;
    }

    // Idempotent re-inject through the same real path: a second Esc keeps
    // the ending state and exit code stable, exactly like the chord.
    d.handle_event(Event::Key(keys::KEY_ESC as u16));
    if !d.session.is_ending() || d.session.exit_code() != 0 {
        io::print_str(
            "[test] FAIL test_logout_protocol_from_chord: esc re-inject changed ending/exit\n",
        );
        return false;
    }

    // Near-miss sweep through the real byte path: Ctrl+Q and plain
    // Backspace must be inert mid-unwind, as with the chord.
    d.handle_event(Event::Key(0x11)); // folds to Ctrl+Q
    d.handle_event(Event::Key(keys::KEY_BACKSPACE_ALT as u16));
    if !d.session.is_ending() || d.session.exit_code() != 0 {
        io::print_str(
            "[test] FAIL test_logout_protocol_from_chord: esc near-miss corrupted the logout\n",
        );
        return false;
    }

    io::print_str("[test] PASS test_logout_protocol_from_chord\n");
    true
}
