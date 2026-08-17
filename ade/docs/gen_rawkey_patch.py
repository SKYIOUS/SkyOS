#!/usr/bin/env python3
"""Generate ade/docs/kernel-rawkey-forward.patch (the section 2.1 un-gate).

Builds the patch by applying the exact edit operations to the REAL kernel
files (byte-preserving, CRLF) and diffing before/after, so line numbers,
context, and line endings are guaranteed to match what `git apply --check`
sees. Same machinery as gen_kernel_patch.py; the result SUPERSEDES
kernel-gui-modifier-delivery.patch (identical A1/A2/A3 base hunks plus the
RawKey forward arm - apply one, not both).

Never touches the kernel tree -- reads it, diffs in memory, writes only the
.patch file into the ade workspace.
"""
import difflib
import io
import os

K = r"C:\Users\nanda\Desktop\Github\SKYIOUS KERNEL"
OUT = os.path.join(r"C:\Users\nanda\Desktop\Github\SkyOS\ade\docs",
                   "kernel-rawkey-forward.patch")

NL = "\n"


def read(p):
    data = open(os.path.join(K, p), "rb").read()
    if data.startswith(b"\xef\xbb\xbf"):  # strip BOM for processing
        data = data[3:]
    return data.decode("utf-8").replace("\r\n", "\n")  # normalize CRLF


def sub_exact(text, old, new, what, count=1):
    n = text.count(old)
    assert n == count, "%s: expected %d occurrence(s), found %d" % (what, count, n)
    return text.replace(old, new)


# ---------------------------------------------------------------------------
# A1 (delivery doc) - kernel/src/main.rs: ctrl/shift tracker arms
# ---------------------------------------------------------------------------
main_rs = read("kernel/src/main.rs")
anchor = (
    "                    0x38 => { comp.alt_held = true; }      // Left Alt make" + NL +
    "                    0xB8 => { comp.alt_held = false; }      // Left Alt break" + NL
)
arms = (
    "                    0x38 => { comp.alt_held = true; }      // Left Alt make" + NL +
    "                    0xB8 => { comp.alt_held = false; }      // Left Alt break" + NL +
    "                    0x1D => { comp.ctrl_held = true; }     // Left Ctrl make" + NL +
    "                    0x9D => { comp.ctrl_held = false; }    // Left Ctrl break" + NL +
    "                    0x2A => { comp.shift_held = true; }    // Left Shift make" + NL +
    "                    0xAA => { comp.shift_held = false; }   // Left Shift break" + NL +
    "                    0x36 => { comp.shift_held = true; }    // Right Shift make" + NL +
    "                    0xB6 => { comp.shift_held = false; }   // Right Shift break" + NL
)
main_new = sub_exact(main_rs, anchor, arms, "A1 tracker anchor")

# ---------------------------------------------------------------------------
# A2 (delivery doc) - kernel/src/gui/mod.rs: fields + init + two forward sites
# ---------------------------------------------------------------------------
mod_rs = read("kernel/src/gui/mod.rs")
fld = (
    "    pub alt_held: bool," + NL +
    "    pub super_held: bool," + NL
)
fld_new = fld + (
    "    pub ctrl_held: bool," + NL +
    "    pub shift_held: bool," + NL
)
mod_new = sub_exact(mod_rs, fld, fld_new, "A2 fields anchor")

init_anchor = (
    "            alt_held: false," + NL +
    "            super_held: false," + NL
)
init_new = init_anchor + (
    "            ctrl_held: false," + NL +
    "            shift_held: false," + NL
)
mod_new = sub_exact(mod_new, init_anchor, init_new, "A2 init anchor")

fwd_old = "                    self.windows[idx].handle_keyboard(key);" + NL
fwd_new = (
    "                    self.windows[idx].handle_keyboard(" + NL +
    "                        key, self.alt_held, self.ctrl_held," + NL +
    "                        self.shift_held, self.super_held," + NL +
    "                    );" + NL
)
mod_new = sub_exact(mod_new, fwd_old, fwd_new, "A2 forward sites", count=2)

# ---------------------------------------------------------------------------
# A3 (delivery doc) - kernel/src/gui/window.rs: queue type + signature
# ---------------------------------------------------------------------------
win_rs = read("kernel/src/gui/window.rs")
q_old = "    pub key_events: VecDeque<u8>," + NL
q_new = (
    "    /// Low byte = char or set-1 scancode; bits 8..11 = alt/ctrl/shift/" + NL +
    "    /// super held. 0 never queued (reserved: \"empty\" at the syscall)." + NL +
    "    pub key_events: VecDeque<u16>," + NL
)
win_new = sub_exact(win_rs, q_old, q_new, "A3 queue type")

sig_old = "    pub fn handle_keyboard(&mut self, key: pc_keyboard::DecodedKey) {" + NL
helper_and_sig = (
    "    /// Pack a scancode/char with the held-modifier bits (bit8=alt, bit9=ctrl," + NL +
    "    /// bit10=shift, bit11=super) and queue it - the shared push for the Unicode" + NL +
    "    /// arm (char) and the RawKey forward arm (set-1 scancode), so both use the" + NL +
    "    /// same packed layout userspace decodes via from_raw." + NL +
    "    fn push_key(&mut self, code: u8, alt: bool, ctrl: bool, shift: bool, super_: bool) {" + NL +
    "        let mut bits = 0u16;" + NL +
    "        if alt { bits |= 1 << 8; }" + NL +
    "        if ctrl { bits |= 1 << 9; }" + NL +
    "        if shift { bits |= 1 << 10; }" + NL +
    "        if super_ { bits |= 1 << 11; }" + NL +
    "        self.key_events.push_back(bits | code as u16);" + NL +
    "    }" + NL +
    NL +
    "    pub fn handle_keyboard(" + NL +
    "        &mut self," + NL +
    "        key: pc_keyboard::DecodedKey," + NL +
    "        alt: bool," + NL +
    "        ctrl: bool," + NL +
    "        shift: bool," + NL +
    "        super_: bool," + NL +
    "    ) {" + NL
)
win_new = sub_exact(win_new, sig_old, helper_and_sig, "A3 signature + push_key helper")

# ---------------------------------------------------------------------------
# B4 (NEW) - the non-terminal RawKey drop arm becomes the section-2.1
# forward set, and the Unicode arm routes through the shared helper.
# ---------------------------------------------------------------------------
arm_old = (
    "                pc_keyboard::DecodedKey::Unicode(c) => {" + NL +
    "                    self.key_events.push_back(c as u8);" + NL +
    "                }" + NL +
    "                pc_keyboard::DecodedKey::RawKey(_k) => {}" + NL
)
arm_new = (
    "                pc_keyboard::DecodedKey::Unicode(c) => {" + NL +
    "                    // Shift is deliberately NOT packed for chars: pc_keyboard" + NL +
    "                    // already folded it into the decoded char (Shift+a -> 'A')," + NL +
    "                    // and ade's KeyEvent::text() rejects shift-modified events," + NL +
    "                    // so a shift bit would stop uppercase typing. The RawKey" + NL +
    "                    // arm below DOES pack shift - arrows/F-keys don't change" + NL +
    "                    // with shift, so the bit is meaningful there. NUL (0x00)" + NL +
    "                    // is skipped: the syscall uses 0 as its empty-queue sentinel." + NL +
    "                    if c != 0 {" + NL +
    "                        debug_assert!(c as u32 <= 0xFF);" + NL +
    "                        self.push_key(c as u8, alt, ctrl, false, super_);" + NL +
    "                    }" + NL +
    "                }" + NL +
    "                pc_keyboard::DecodedKey::RawKey(k) => match k {" + NL +
    "                    // RawKey forward set (kernel-keyboard-gate.md section 2.1):" + NL +
    "                    // the E0-extended arrows (set-1 second byte) + F11/F12 are" + NL +
    "                    // queued as `scancode | mods<<8` - the same packed layout as" + NL +
    "                    // the Unicode arm - so a11y arrow navigation and the F11/F12" + NL +
    "                    // debug-overlay keys work from real hardware. Escape is" + NL +
    "                    // deliberately NOT forwarded: its set-1 code 0x01 collides" + NL +
    "                    // with Ctrl+A in from_byte, and Esc already flows as Unicode" + NL +
    "                    // 0x1B (us104.rs:23). Everything else (numpad block, nav" + NL +
    "                    // block, media keys) stays dropped." + NL +
    "                    pc_keyboard::KeyCode::ArrowUp => {" + NL +
    "                        self.push_key(0x48, alt, ctrl, shift, super_)" + NL +
    "                    }" + NL +
    "                    pc_keyboard::KeyCode::ArrowDown => {" + NL +
    "                        self.push_key(0x50, alt, ctrl, shift, super_)" + NL +
    "                    }" + NL +
    "                    pc_keyboard::KeyCode::ArrowLeft => {" + NL +
    "                        self.push_key(0x4B, alt, ctrl, shift, super_)" + NL +
    "                    }" + NL +
    "                    pc_keyboard::KeyCode::ArrowRight => {" + NL +
    "                        self.push_key(0x4D, alt, ctrl, shift, super_)" + NL +
    "                    }" + NL +
    "                    pc_keyboard::KeyCode::F11 => self.push_key(0x57, alt, ctrl, shift, super_)," + NL +
    "                    pc_keyboard::KeyCode::F12 => self.push_key(0x58, alt, ctrl, shift, super_)," + NL +
    "                    _ => {}" + NL +
    "                }," + NL
)
win_new = sub_exact(win_new, arm_old, arm_new, "B4 RawKey forward arm")


def make_patch(path, old, new):
    diff = difflib.unified_diff(
        old.splitlines(keepends=True),
        new.splitlines(keepends=True),
        fromfile="a/" + path,
        tofile="b/" + path,
        n=3,
    )
    out = ["diff --git a/%s b/%s" % (path, path)]
    for line in diff:
        out.append(line.rstrip(NL))
    return NL.join(out)


parts = [
    "# Kernel RawKey forward set - section 2.1 un-gate, generated patch",
    "#",
    "# Source specs: ade/docs/kernel-keyboard-gate.md section 2.1 (the KeyCode ->",
    "#   set-1 scancode table) and ade/docs/kernel-gui-modifier-delivery.md",
    "#   (Design A bit layout). Host-pinned userspace side:",
    "#   tests/test_login_flow.py::TestKernelKeyContract + TestScancodeConstants;",
    "#   low byte = char or set-1 scancode; bit8 = alt; bit9 = ctrl; bit10 = shift;",
    "#   bit11 = super.",
    "#",
    "# SUPERSEDES kernel-gui-modifier-delivery.patch: the A1/A2/A3 base hunks",
    "#   below are byte-identical to that patch's, and the non-terminal RawKey",
    "#   drop arm is replaced by the section-2.1 forward set. Apply THIS patch,",
    "#   not both.",
    "#",
    "# A1: ctrl/shift tracker arms in gui_refresh_task (kernel/src/main.rs).",
    "# A2: Compositor ctrl_held/shift_held fields + init + 2 forward sites",
    "#     (kernel/src/gui/mod.rs) - the RawKey arm needs the held modifiers,",
    "#     and only the trackers produce them.",
    "# A3: Window key_events VecDeque<u8> -> VecDeque<u16> and handle_keyboard",
    "#     gains the four held-modifier bools - `byte | mods<<8` cannot fit a",
    "#     u8 queue, so the widening is a hard dependency of the packed push.",
    "#     The packing is factored into push_key() (shared by both arms) instead",
    "#     of A3's inline duplication.",
    "# B4: THE NEW ARM - the non-terminal `RawKey(_k) => {}` becomes a match:",
    "#     ArrowUp/Down/Left/Right (E0 second bytes 0x48/0x50/0x4B/0x4D) and",
    "#     F11/F12 (0x57/0x58) are queued as `scancode | mods<<8`; Escape and",
    "#     everything else stay dropped (Esc already flows as Unicode 0x1B, and",
    "#     its set-1 code 0x01 collides with Ctrl+A in from_byte). The terminal-",
    "#     window RawKey arm is untouched (sash input is a separate path).",
    "#",
    "# Userspace consumers (landed and pinned in ade):",
    "#   - plain arrows 72/75/77/80 drive the a11y ring (handle_a11y_key SCAN_*",
    "#     arms; test_a11y_arrows_from_byte_stream) - the E0 second byte, exactly",
    "#     the code the queue must carry (the pc_keyboard state machine consumed",
    "#     the 0xE0 prefix, so the window never sees it);",
    "#   - shifted/ctrl/alt arrows arrive with modifier bits and are inert by",
    "#     design (a11y modifier guard; pinned in the same test);",
    "#   - F11 0x57 lands on the SCAN_F11 binding (ToggleDebugOverlay); F12 0x58",
    "#     lands on the KEY_X binding ('X' = 0x58) - the legacy overlay toggles",
    "#     on both.",
    "#",
    "# Shift bit handling (review-verified against ade/src/input/mod.rs:130):",
    "#   the Unicode arm does NOT pack shift - pc_keyboard already folds shift",
    "#   into the decoded char, and KeyEvent::text() rejects shift-modified",
    "#   events, so packing it would silently stop uppercase typing. The RawKey",
    "#   arm packs all four bits because arrows/F-keys don't change with shift.",
    "#",
    "# Integration points verified against the current kernel tree (a18848f):",
    "#   - focused_window() is Some(len-1) whenever windows is non-empty, so the",
    "#     forward reaches the topmost window even on the empty desktop - as",
    "#     long as ade keeps its desktop root window registered.",
    "#   - kbd.add_byte(scancode) runs unconditionally AFTER the tracker match",
    "#     (kernel/src/main.rs gui_refresh_task), so the 0xE0 extended prefix",
    "#     still reaches pc_keyboard's state machine - 0xE0 0x48 decodes as",
    "#     ArrowUp, not Keypad8.",
    "#",
    "# Landing step: git apply --check passed; run a kernel `cargo check` after",
    "#   applying - apply cannot prove types (the two-phase borrows at the A2",
    "#   forward sites, and the pc_keyboard::KeyCode variant names - ArrowUp/",
    "#   ArrowDown/ArrowLeft/ArrowRight/F11/F12 - which are unverifiable host-",
    "#   side because the crate is not vendored in the repo tree).",
    "#",
    "# Generate: python3 gen_rawkey_patch.py",
    "# Verify:  cd 'SKYIOUS KERNEL' && git apply --check kernel-rawkey-forward.patch",
    "#          (LF patch; verified against the CRLF worktree on Aug 12, 2026)",
    "# Apply:   git apply kernel-rawkey-forward.patch",
    "",
    make_patch("kernel/src/main.rs", main_rs, main_new),
    make_patch("kernel/src/gui/mod.rs", mod_rs, mod_new),
    make_patch("kernel/src/gui/window.rs", win_rs, win_new),
    "",
]

# Emitted LF (the SkyOS repo is LF; `git apply` accepts LF against the CRLF
# kernel worktree - same verified behavior as the delivery patch).
body = NL.join(parts)

io.open(OUT, "w", encoding="utf-8", newline="").write(body)
print("WROTE", OUT, len(body), "bytes")
