# Kernel GUI modifier delivery — unblock design for the Phase C session-end chord

**Status:** SPEC ONLY — no kernel code changed. The kernel is mid-major-change
(this doc's anchors are verified against the current tree at
`SKYIOUS KERNEL/kernel/src/`, Aug 9, 2026; function names are the stable
anchors if line numbers drift).

**Landed (Aug 10, 2026) — the userspace half only:** A5 (`libsarga`
`get_key() -> Option<u16>`, all non-ade GUI consumers truncate `as u8` at
their call sites) and A6 (`ade` `KeyEvent::from_raw`, `Event::Key(u16)`, a11y
modifier guard) are in the tree and pinned by the ade selftest
`test_from_raw` plus the host test
`tests/test_login_flow.py::TestKernelKeyContract` (a Python port of the
decode, so the libsarga/input half cannot drift while the kernel rewrite is
in flight). A1–A4 (kernel) are unchanged and still required for the chord.
**Purpose:** unblock the Phase C gate — deliver Alt so a real hardware
Ctrl+Alt+Backspace reaches ade. Note (Aug 9, 2026): the Phase B harness no
longer blocks on this — Esc on an empty desktop is a byte-deliverable second
session-end path (0x1B is the one control byte the stream carries), so
`tests/qemu_gui_login.exp` sends `esc` and observes `[ade] session ended` +
init respawn on today's kernel. This spec remains the upgrade path for the
chord (and all future Alt/Ctrl/Shift combinations) when the kernel settles.
**Companion:** the hop-by-hop chain is documented in
`docs/session-lifecycle.md` §"The raw-key → byte input path". This doc is the
concrete, reviewable change set that section points to.

---

## 1. The problem (verified, not assumed)

Every key becomes a **single byte with zero modifier bits** before it leaves
the kernel. The kernel's compositor *does* hold modifier state — but only for
its own shortcuts (Alt+F4, Alt+Tab, Super+arrow), and it never puts that state
into the byte stream ade reads.

The chain (all anchors read from the current tree):

| Hop | Site (kernel/src/) | What happens |
|---|---|---|
| IRQ1 → scancode | `interrupts.rs:647` → `keyboard.rs:7` → `task/keyboard.rs:17` `add_scancode` | scancode byte → `GUI_SCANCODE_QUEUE` |
| Decode | `main.rs:421` `gui_refresh_task` (spawned `:417`) | two decoders on the same scancode: (a) raw-scancode modifier tracker `:437-441` (`0x38/0xB8` → `alt_held`; `0xE0` no-op; `0x5B/0xDB` → `super_held`), (b) `kbd.add_byte`/`process_keyevent` (`:455-456`, `HandleControl::Ignore` at `:428`) → `comp.handle_keyboard(key)` (`:457`) |
| Compositor | `gui/mod.rs:580` `handle_keyboard` | RawKey arm `:582-596` (Alt+F4, then forward); Unicode arm `:599-671` (Alt+Ctrl+0x04, Super+arrow, Alt+Tab, then fall-through forward `:670`) |
| Window | `gui/window.rs:190` `handle_keyboard` | non-terminal branch `:203-211`: **`self.key_events.push_back(c as u8)` (`:207`)** — the char becomes a byte, modifiers dropped; RawKey dropped (`:209`) |
| Syscall | `syscalls/mod.rs:4792` `sys_gui_get_key` (dispatch `:678`) | `win.key_events.pop_front().map(|k| k as u64).unwrap_or(0)` — **already `u64`, so widening a key costs zero syscall changes** |
| Userspace | `libsarga/src/gui.rs:464` `get_key` | `Some(k as u8)` (`:470`) — truncation |
| ade | `ade/src/input/mod.rs:77-93` `from_byte` | folds `0x01..=0x1A` → Ctrl+letter; **forces `alt`/`shift` false** |

Consequence for the chord: pc-keyboard maps Backspace to `Unicode(0x08)`
unconditionally (`us104.rs:108`), so Ctrl+Alt+Backspace arrives at
`handle_keyboard` as `Unicode(0x08)` while Ctrl/Alt live **only** in the
compositor's `alt_held` state (`0x38` make tracking). ade receives plain
`0x08` (edit Backspace) and the chord never fires.

Two gaps in the tracker to close for *any* design:
1. **`ctrl_held`/`shift_held` don't exist.** `Compositor` has only
   `alt_held`/`super_held` (`gui/mod.rs:50-51`). Ctrl+Alt+Backspace needs Ctrl.
2. `HandleControl::Ignore` (`main.rs:428`) means Ctrl+letter decodes to the
   plain letter — irrelevant for the chord (we track Ctrl by raw scancode),
   but relevant if you ever want Ctrl+letter shortcuts to reach ade as
   `0x01..=0x1A`. Not required here; noted as an orthogonal decision.

Also verified: right-hand Ctrl/Alt are `E0 1D`/`E0 38` in set 1 — the `0xE0`
no-op arm plus the standalone `0x1D`/`0x38` arms make the left-hand arms catch
them anyway (the E0 byte pops first and is ignored, the 1D/38 byte then hits
the plain arm). So **no extended-prefix state machine is needed** for ctrl/alt.

---

## 2. Design A (recommended): deliver modifier bits in a 16-bit key

> **Verbatim patch available (Aug 12, 2026):** `docs/kernel-gui-modifier-delivery.patch`
> — the A1–A4 changes below, generated from the actual kernel sources and
> verified with `git apply --check` against the CRLF worktree. Regenerate
> (after the kernel drifts) with `python3 docs/gen_kernel_patch.py`.

One mechanism unblocks the chord **and** every future Alt/Ctrl/Shift combo
(Alt+Tab confirm, Ctrl+Shift+S, etc.) for free — and the syscall surface is
untouched because `sys_gui_get_key` already returns `u64`.

**Key layout:** low byte = the character; bits 8..11 = alt/ctrl/shift/super
held. `0` stays reserved for "queue empty" (pc-keyboard never produces NUL).

### Change A1 — `kernel/src/main.rs` tracker (≈8 lines)

In `gui_refresh_task`, extend the raw-scancode match at `:437-441`:

```rust
                match scancode {
                    0x38 => { comp.alt_held = true; }      // Left Alt make
                    0xB8 => { comp.alt_held = false; }     // Left Alt break
+                    0x1D => { comp.ctrl_held = true; }     // Left Ctrl make
+                    0x9D => { comp.ctrl_held = false; }    // Left Ctrl break
+                    0x2A => { comp.shift_held = true; }    // Left Shift make
+                    0xAA => { comp.shift_held = false; }   // Left Shift break
+                    0x36 => { comp.shift_held = true; }    // Right Shift make
+                    0xB6 => { comp.shift_held = false; }   // Right Shift break
                    0xE0 => { /* Extended prefix — next byte is the real scancode */ }
```

### Change A2 — `kernel/src/gui/mod.rs` Compositor (2 fields + 2 forward sites)

Fields next to `alt_held` (`:50-51`):

```rust
    pub alt_held: bool,
    pub super_held: bool,
+    pub ctrl_held: bool,
+    pub shift_held: bool,
```

Init in `Compositor::new` (next to `alt_held: false` at `:136`):
`ctrl_held: false, shift_held: false,`

Forward the state at both places `handle_keyboard` calls the window
(RawKey arm `:596` and the Unicode fall-through `:670`):

```rust
-                    self.windows[idx].handle_keyboard(key);
+                    self.windows[idx].handle_keyboard(
+                        key, self.alt_held, self.ctrl_held, self.shift_held, self.super_held,
+                    );
```

### Change A3 — `kernel/src/gui/window.rs` (queue type + packing)

```rust
-    pub key_events: VecDeque<u8>,
+    /// Low byte = char; bits 8..11 = alt/ctrl/shift/super held. 0 never
+    /// queued (reserved: "empty" at the syscall).
+    pub key_events: VecDeque<u16>,
```

```rust
-    pub fn handle_keyboard(&mut self, key: pc_keyboard::DecodedKey) {
+    pub fn handle_keyboard(
+        &mut self,
+        key: pc_keyboard::DecodedKey,
+        alt: bool,
+        ctrl: bool,
+        shift: bool,
+        super_: bool,
+    ) {
```

Non-terminal branch (`:203-211`):

```rust
                 pc_keyboard::DecodedKey::Unicode(c) => {
-                    self.key_events.push_back(c as u8);
+                    let mut bits = 0u16;
+                    if alt { bits |= 1 << 8; }
+                    if ctrl { bits |= 1 << 9; }
+                    if shift { bits |= 1 << 10; }
+                    if super_ { bits |= 1 << 11; }
+                    self.key_events.push_back(bits | c as u16);
                 }
```

The kernel-internal terminal branch (`:191-200`) feeds `term.handle_char`
and ignores the new args — same function, so it inherits the signature, but
the params are simply unused there (no `_`-prefix needed, the branch never
reads them). RawKey arms still drop.

### Change A4 — `kernel/src/syscalls/mod.rs` — **no change**

`sys_gui_get_key` (`:4792`) already does `.map(|k| k as u64)`; u16 → u64 is
lossless. Dispatch (`:678`) untouched.

### Change A5 — `libsarga/src/gui.rs` (userspace, doable immediately)

```rust
-    pub fn get_key(&mut self) -> Option<u8> {
+    /// Low byte = char; bits 8..11 = alt/ctrl/shift/super held (0 until the
+    /// kernel delivers them — additive; high bits arrive as zero today).
+    pub fn get_key(&mut self) -> Option<u16> {
         let k = unsafe { syscall1(SYS_GUI_GET_KEY, self.id) };
         if k == 0 {
             None
         } else {
-            Some(k as u8)
+            Some(k as u16)
         }
     }
```

### Change A6 — `ade/src/input/mod.rs` (userspace, doable immediately)

```rust
+    /// Decode a kernel key value (low byte = char, bits 8..11 = modifier
+    /// held). When the high bits are zero this is exactly `from_byte` — so
+    /// the change is additive and the current byte-stream behavior is
+    /// preserved until the kernel sends bits.
+    pub fn from_raw(raw: u16) -> KeyEvent {
+        let mut ev = Self::from_byte((raw & 0xFF) as u8);
+        if raw & (1 << 8) != 0 { ev.alt = true; }
+        if raw & (1 << 9) != 0 { ev.ctrl = true; }
+        if raw & (1 << 10) != 0 { ev.shift = true; }
+        ev
+    }
```

Wiring: `ade/src/main.rs:65` `while let Some(key) = desktop_win.get_key()` —
`key` becomes `u16`; the `Event::Key` payload widens to `u16` and
`handle_key_event_raw` (`desktop.rs:807`) calls `KeyEvent::from_raw(key)`
instead of `from_byte`. **No routing-table change needed:** the Phase 3 keymap
(`resolve`) already matches `KeyEvent { code, ctrl, alt, shift }` on all three
bits, and `test_session_end_gate` already pins `Ctrl+Alt+Backspace →
KeyAction::Quit`.

### Why this design

- The syscall is already `u64` — zero ABI churn.
- ade's `KeyEvent`/keymap was deliberately built for exactly this shape
  (Phase 3, "input extraction" — `resolve` matches on all three bits).
- One mechanism covers the chord, Alt+Tab confirmation, and any future chord;
  no one-off sentinel bytes to remember.
- The userspace half (A5+A6) can land now and is provably inert until the
  kernel sends bits (high bits are zero → `from_raw` ≡ `from_byte`).

---

## 3. Design B (minimal): encode the chord as a distinct byte

If the kernel team wants a ~6-line change instead of a queue-type widening:

### Change B1+B2 — same as A1 (ctrl/shift tracking) + A2 fields
(ctrl_held/shift_held fields + tracker arms; no forwarding needed.)

### Change B3 — `kernel/src/gui/mod.rs`, Unicode arm

Insert before the Alt+Tab block (after the Super+arrow block, ~`:635`):

```rust
+                // Ctrl+Alt+Backspace: encode the chord as a sentinel byte
+                // (0x1E) the focused window forwards to userspace. ade's
+                // session-end key (KeyAction::Quit) is the only consumer.
+                if self.alt_held && self.ctrl_held && c == '\u{0008}' {
+                    if let Some(idx) = self.focused_window() {
+                        self.windows[idx].key_events.push_back(0x1E);
+                    }
+                    self.alt_held = false;
+                    return;
+                }
```

Sentinel rationale: `0x1E` is unmapped in ade's byte decode — not a Ctrl fold
(`0x01..=0x1A`), not a special (`0x08/0x0A/0x0D/0x1B/0x7F`), not printable
(`0x20..=0x7E`). It is inert today; if Design A later lands, the high bits
make the sentinel arm unreachable and it can be deleted.

### Change B4 — `ade/src/input/mod.rs`

```rust
    pub const KEY_SESSION_END: u8 = 0x1E;   // kernel-encoded Ctrl+Alt+Backspace
```

```rust
    pub fn from_byte(b: u8) -> KeyEvent {
+        if b == keys::KEY_SESSION_END {
+            return KeyEvent::new(keys::KEY_BACKSPACE, true, true, false);
+        }
        let (code, ctrl) = match b {
```

### Trade-off vs A

| | A (u16 bits) | B (sentinel) |
|---|---|---|
| Kernel lines | ~20, type widening | ~8, two fields + one arm |
| Unblocks future chords | yes, all | only the one encoded |
| Syscall / queue churn | queue type + forward args | none |
| ade side | `from_raw` + Event widening | one `from_byte` arm |
| Risk | mechanical; tested by `from_raw` pin | sentinel collides if a future `MapLettersToControl` maps 0x1E (Ctrl+^) — then use 0x7F-adjacent or scan range |

**Recommendation: A.** It is what ade's keymap was built for, the syscall
already returns `u64`, and it retires the whole class of "modifiers never
reach userspace" bugs instead of whack-a-moling one chord. B is the right
answer only if the kernel team wants the smallest possible diff this cycle.

---

## 4. Verification plan (after the kernel change lands)

1. **Unit (userspace, A5/A6 landed Aug 10, 2026):** `test_from_raw` pins
   `from_raw` round-trips in `testing/input.rs` — plain `0x63` →
   `('c', 0,0,0)`; the chord `0x0308` (note: `0x08 | alt<<8 | ctrl<<9`,
   NOT the `0x0108` of an earlier draft — bit8 is alt, bit9 is ctrl) →
   `(Backspace, ctrl=true, alt=true)`; partial-modifier decodes
   (`0x0108` = alt-only, `0x0208` = ctrl-only) pin the bit order; and the
   host suite `tests/test_login_flow.py::TestKernelKeyContract` mirrors the
   same values in Python. Existing `test_session_end_gate`
   already asserts the chord → `KeyAction::Quit`. On top of that (Aug 10,
   2026): `test_keymap` sweeps the packed space — every one of the 18
   binding rows must be reachable via `from_raw` over (byte, mods) pairs, so
   nothing stays synthetic once the kernel lands — and `test_session_end_gate`
   drives the chord through the real `Event::Key(0x0308)` path (proving the
   a11y modifier guard cannot swallow it) with `0x0108`/`0x0208`
   partial-modifier negatives pinning the bit order.
2. **Boot probe (login half, today):** `tests/probe_sendkey.py` (no-expect)
   boots a fresh kernel+ISO and drives the login (`sendkey r o o t` / `tab` /
   `s k y o s` / `ret`), proving sendkey reaches login-manager and
   `[ade] session established` appears. It does not send the chord; proving
   the chord needs an added `sendkey ctrl-alt-backspace` + `[KBD] IRQ1
   fired!` + `[ade] session ended` sequence once this change lands.
3. **Phase B harness — byte-deliverable today (Aug 9, 2026):**
   `tests/qemu_gui_login.exp` (expect-based) drives the GUI login
   `root`/`skyos` → waits for `[ade] session established` → sends `esc` (the
   Esc-on-empty-desktop session-end path) → asserts `[ade] session ended`
   and the init respawn line (`[init] starting service: login-manager`).
   After this change lands, the same harness can additionally send
   `ctrl-alt-backspace` to prove the chord.
4. The kernel CI `gui-gate` job already asserts GUI reachability on every
   build; this adds the logout half.

## 5. Assumptions / open questions

- **Kernel is mid-major-change:** anchors verified Aug 9, 2026 against
  `SKYIOUS KERNEL/kernel/src/`; function names (`gui_refresh_task`,
  `Compositor::handle_keyboard`, `Window::handle_keyboard`,
  `sys_gui_get_key`) are the stable contract, line numbers may drift.
- **Raw-scancode tracker sees every scancode:** the `0xE0` extended prefix is
  a separate queue entry and is ignored — verified that left-arm codes still
  catch right-hand Ctrl/Alt (E0-then-1D pops as two bytes). If a future
  driver coalesces the pair, the tracker needs a one-byte lookahead. The
  same split-pop caveat applies to `0xE1`: set-1 Pause sends `E1 1D 45 E1 9D
  C5`, so the new `0x1D`/`0x9D` arms fire transiently during Pause
  (self-correcting on break — same class as the pre-existing `0x38`
  exposure); not worth state, noted for completeness.
- **Super-arrow bug (adjacent, not blocking):** `super_held` is set by
  `0x5B/0xDB` which in set 1 are E0-prefixed — consistent with the current
  split-pop model, so it likely works; unverified at runtime. Not required
  for the chord.
- **Ctrl as letter (orthogonal):** keeping `HandleControl::Ignore` means
  Ctrl+letter shortcuts (Ctrl+W close, Ctrl+C to terminal) stay
  dead-as-text today — a separate decision from the chord; Design A would
  make the future `MapLettersToControl` switch automatic on the ade side
  (`from_byte` already folds `0x01..=0x1A`).
- **Queue lifetime:** a window closed between `push_back` and the user's
  `sys_gui_get_key` drops the event — pre-existing, unchanged by either
  design.
- **Open:** should the kernel also deliver modifier state on *release*
  (for key-up semantics)? Not needed for any current chord — no change
  proposed.
