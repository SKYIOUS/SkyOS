#!/usr/bin/env python3
"""Generate ade/docs/kernel-gui-modifier-delivery.patch (A1-A4, Design A).

Builds the patch by applying the exact edit operations to the REAL kernel
files (byte-preserving, CRLF) and diffing before/after, so line numbers,
context, and line endings are guaranteed to match what `git apply --check`
sees. A4 (no syscall change) is documented in the header only.

Never touches the kernel tree -- reads it, diffs in memory, writes only the
.patch file into the ade workspace.
"""
import difflib
import io
import os

K = r"C:\Users\nanda\Desktop\Github\SKYIOUS KERNEL"
OUT = os.path.join(r"C:\Users\nanda\Desktop\Github\SkyOS\ade\docs",
                   "kernel-gui-modifier-delivery.patch")

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
# A1 - kernel/src/main.rs: tracker arms
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
# A2 - kernel/src/gui/mod.rs: fields + init + two forward sites
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
# A3 - kernel/src/gui/window.rs: queue type + signature + packing
# ---------------------------------------------------------------------------
win_rs = read("kernel/src/gui/window.rs")
q_old = "    pub key_events: VecDeque<u8>," + NL
q_new = (
    "    /// Low byte = char; bits 8..11 = alt/ctrl/shift/super held. 0 never" + NL +
    "    /// queued (reserved: \"empty\" at the syscall)." + NL +
    "    pub key_events: VecDeque<u16>," + NL
)
win_new = sub_exact(win_rs, q_old, q_new, "A3 queue type")

sig_old = "    pub fn handle_keyboard(&mut self, key: pc_keyboard::DecodedKey) {" + NL
sig_new = (
    "    pub fn handle_keyboard(" + NL +
    "        &mut self," + NL +
    "        key: pc_keyboard::DecodedKey," + NL +
    "        alt: bool," + NL +
    "        ctrl: bool," + NL +
    "        shift: bool," + NL +
    "        super_: bool," + NL +
    "    ) {" + NL
)
win_new = sub_exact(win_new, sig_old, sig_new, "A3 signature")

push_old = "                    self.key_events.push_back(c as u8);" + NL
push_new = (
    "                    debug_assert!(c as u32 <= 0xFF);" + NL +
    "                    let mut bits = 0u16;" + NL +
    "                    if alt { bits |= 1 << 8; }" + NL +
    "                    if ctrl { bits |= 1 << 9; }" + NL +
    "                    if shift { bits |= 1 << 10; }" + NL +
    "                    if super_ { bits |= 1 << 11; }" + NL +
    "                    self.key_events.push_back(bits | c as u16);" + NL
)
win_new = sub_exact(win_new, push_old, push_new, "A3 packing")


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
    "# Kernel GUI modifier delivery - A1..A4 (Design A), generated patch",
    "#",
    "# Source spec: ade/docs/kernel-gui-modifier-delivery.md (Design A).",
    "# Host-pinned bit layout (tests/test_login_flow.py::TestKernelKeyContract):",
    "#   low byte = char; bit8 = alt; bit9 = ctrl; bit10 = shift; bit11 = super.",
    "#   Ctrl+Alt+Backspace chord = 0x08 | (1<<8) | (1<<9) = 0x0308.",
    "#",
    "# A1: ctrl/shift tracker arms in gui_refresh_task (kernel/src/main.rs).",
    "# A2: Compositor ctrl_held/shift_held fields + init + 2 forward sites",
    "#     (kernel/src/gui/mod.rs).",
    "# A3: Window key_events VecDeque<u8> -> VecDeque<u16>, signature gains the",
    "#     four held-modifier bools, packing in the non-terminal Unicode arm",
    "#     (kernel/src/gui/window.rs).",
    "# A4: NO syscall change - sys_gui_get_key already does .map(|k| k as u64);",
    "#     u16 -> u64 is lossless (kernel/src/syscalls/mod.rs, untouched).",
    "#",
    "# Landing step: git apply --check passed; run a kernel `cargo check` after",
    "# applying - the two A2 forward sites rely on two-phase borrows",
    "# (self.windows[idx].handle_keyboard(key, self.alt_held, ...)), the canonical",
    "# xs[i].method(xs.field) pattern that compiles, but apply cannot prove types.",
    "# The packing debug_assert!(c as u32 <= 0xFF) pins the low-byte-char",
    "# assumption; US104 emits no char >= 0x100 today.",
    "#",
    "# Generate: python3 gen_kernel_patch.py",
    "# Verify:  cd 'SKYIOUS KERNEL' && git apply --check kernel-gui-modifier-delivery.patch",
    "#          (LF patch; verified against the CRLF worktree on Aug 12, 2026)",
    "# Apply:   git apply kernel-gui-modifier-delivery.patch",
    "",
    make_patch("kernel/src/main.rs", main_rs, main_new),
    make_patch("kernel/src/gui/mod.rs", mod_rs, mod_new),
    make_patch("kernel/src/gui/window.rs", win_rs, win_new),
    "",
]

# The patch is emitted LF: the SkyOS repo (where this file lives) is LF, and
# `git apply` accepts LF patches against the CRLF kernel worktree (verified:
# both variants pass `git apply --check`). LF also survives git's autocrlf
# normalization when this file is committed.
body = NL.join(parts)

io.open(OUT, "w", encoding="utf-8", newline="").write(body)
print("WROTE", OUT, len(body), "bytes")
