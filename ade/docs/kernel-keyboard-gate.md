# Kernel keyboard gate — evidence map for the GUI input rewrite

Status: evidence doc for the kernel rewrite (kernel is mid-major-change; treat
function names + syscall numbers as the stable anchors, line numbers as of the
checkout in `kernel/kernel/` at time of writing, Aug 8 2026).

> **Queue:** this is **K2** in `session-lifecycle.md` §6 Kernel change queue —
> the rewrite's consolidated landing checklist (mods byte, control-letter
> semantics, and the exact harness conditions for Phase B + the chord).

Scope: the two userspace features gated on kernel keyboard delivery —

1. **Phase B Tab/Enter** — driving `login-manager`'s login form (Tab switches
   field, Enter submits) from CI via QEMU monitor `sendkey`.
2. **Ctrl+Alt+Backspace session-end chord** — `ade`'s only session-end key
   (pinned in `ade/src/util/testing/input.rs`, keymap in `ade/src/input/mod.rs`).

---

## 1. The verified input chain (top to bottom)

| # | Stage | Location | What it does |
|---|---|---|---|
| 1 | PS/2 IRQ1 | `kernel/kernel/src/interrupts.rs:647-672` | `keyboard_interrupt_handler`: drains port `0x60`; one-shot `[KBD] IRQ1 fired!` marker (line 655); feeds `crate::keyboard::handle_scancode(byte)` |
| 2 | IRQ→queue | `kernel/kernel/src/interrupts.rs:636,670` → `kernel/kernel/src/keyboard.rs:7` → `kernel/kernel/src/task/keyboard.rs:17-31` | `add_scancode` pushes to `GUI_SCANCODE_QUEUE` (capacity 100); `try_pop_scancode` (line 30-31) pops for the GUI |
| 3 | Decode task | `kernel/kernel/src/main.rs:422-470` `gui_refresh_task` | 100 Hz loop: `try_pop_scancode()` → **`Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)`** (line **428**) → `kbd.add_byte` → `kbd.process_keyevent` → `comp.handle_keyboard(key)`. Also tracks **Alt/Super from raw scancodes** (lines 432-445: `0x38/0xB8` Left-Alt, `0xE0` prefix, `0x5B/0xDB` Win) |
| 4 | Compositor dispatch | `kernel/kernel/src/gui/mod.rs:571-675` `Compositor::handle_keyboard(DecodedKey)` | `RawKey` arm (line **582**): only F4+alt close, then forwards to focused window. `Unicode` arm (line **596**): Alt+Ctrl close, Super+arrow snap, Alt+Tab cycle, Esc-cancel, then forwards (line 671-674) |
| 5 | Window sink | `kernel/kernel/src/gui/window.rs:194-222` `Window::handle_keyboard` | Terminal: Unicode→`term.handle_char`; **`RawKey(_k) => {}` dropped (line 202)**. Non-terminal: Unicode→`self.key_events.push_back(c as u8)` (line **207**); **`RawKey(_k) => {}` dropped (line 209)** |
| 6 | Queue | `kernel/kernel/src/gui/window.rs:26` | `pub key_events: VecDeque<u8>` — **one byte, no modifier bits** |
| 7 | Syscall | `kernel/kernel/src/syscalls/mod.rs:4792-4797` `sys_gui_get_key` | pops one byte (`k as u64`), `0` = empty |
| 8 | Userspace | `libsarga/src/gui.rs:464-471` `Window::get_key` | returns that byte; **no folding, no modifiers** |

Userspace contract the kernel must serve (`ade/src/input/mod.rs`):
`KeyEvent::from_byte` treats `0x01..=0x1A` as Ctrl+letter, `0x09`=Tab,
`0x0A/0x0D`=Enter, `0x7F`/`0x08`=Backspace, scan codes 72/75/77/80 = arrows,
`0x57` = F11. `alt`/`shift` are **hard-coded false** today because the byte
stream cannot carry them.

---

## 2. Where RawKey is dropped — and the correction

The two drop points are `gui/window.rs:202` (terminal) and `gui/window.rs:209`
(non-terminal): every `DecodedKey::RawKey` is discarded. That gates arrows and
F-keys (not in scope for the two features).

**Correction to the premise:** with `HandleControl::Ignore`, **Ctrl+letter does
NOT arrive as RawKey**. In pc-keyboard 0.5.1 the letter arms only consult the
ctrl bit when `MapLettersToUnicode` is set:

```rust
// vendored pc-keyboard-0.5.1/src/layouts/us104.rs:14
let map_to_unicode = handle_ctrl == HandleControl::MapLettersToUnicode;
// ...:221 KeyCode::A arm
if map_to_unicode && modifiers.is_ctrl() { DecodedKey::Unicode('\u{0001}') }
else if modifiers.is_caps() { DecodedKey::Unicode('A') }
else { DecodedKey::Unicode('a') }
```

So under `Ignore`, Ctrl+A degrades to the **plain letter `'a'` (0x61)** and is
pushed to `key_events` as text — every Ctrl+letter desktop shortcut is
currently DOA through the GUI path, silently becoming a keystroke. That, not
RawKey, is the operative bug for shortcuts.

`HandleControl` in 0.5.1 has exactly two variants (`lib.rs:200-207`):
`MapLettersToUnicode` (letters → U+0001..U+001A) and `Ignore`. There is **no
`MapLettersToControl` in this version** — that name is from older pc_keyboard
releases; the doc below uses the 0.5.1 name.

### What the current kernel delivers vs. what userspace expects

| Key | Kernel delivers today | Userspace expects (`input/mod.rs`) | Verdict |
|---|---|---|---|
| Tab | `Unicode('\t')` = 0x09 (`us104.rs:109`) | 0x09 → Tab / login-manager field switch | ✅ flows |
| Enter | `Unicode('\n')` = 0x0A (`us104.rs:317`) | 0x0A/0x0D → Enter | ✅ flows |
| Backspace | `Unicode(0x08)` (`us104.rs:108`) | 0x7F or 0x08 → Backspace (`KEY_BACKSPACE_ALT`) | ✅ flows (normalized) |
| Ctrl+letter | plain letter (`us104.rs:221+`) | control code 0x01..0x1A with ctrl=true | ❌ degraded to text |
| Ctrl+Alt+Backspace | 0x08, no modifier bits | `KeyEvent(0x7F, ctrl+alt)` → `Quit` | ❌ **blocked — no mods byte** |
| Arrows / F-keys | RawKey → dropped (`window.rs:202,209`) | SCAN_UP=72 … SCAN_F11=0x57 | ❌ dropped — un-gate table in §2.1 |

### 2.1 The KeyCode → set-1 scancode table the RawKey path needs

The compositor already forwards every non-F4 `RawKey` to the focused window
(`gui/mod.rs:582-596`); the drop is the `RawKey(_k) => {}` arm at
`gui/window.rs:209` (non-terminal). Replacing that arm with a scancode push
needs the inverse of pc-keyboard 0.5.1's `map_scancode` /
`map_extended_scancode` (`src/scancodes.rs`) — `KeyCode` → set-1 make byte.
All values below are read from that source (vendored at
`~/.cargo/registry/src/index.crates.io-*/pc-keyboard-0.5.1`, verified
Aug 10, 2026).

**Key finding: the arrows are E0-extended scancodes.** In set 1 the arrow
keys are two-byte sequences (`E0 48` etc.), and the single-byte codes
`0x47..=0x53` are the numpad block (`0x48` alone = Numpad8). pc-keyboard's
state machine consumes the `E0` prefix internally (`scancodes.rs`
`advance_state`), so the window receives a decoded `KeyCode::ArrowUp` and
never sees the E0 — the table below is the E0 **second byte**, which is what
the queue must carry. Numpad keys with numlock OFF decode to the same arrow
`KeyCode`s (`us104.rs:414-419`), so pushing the E0 byte is semantically
correct there too; with numlock ON they are Unicode digits and never reach
the RawKey arm.

| `pc_keyboard::KeyCode` | Set-1 make code | Kind | Userspace constant (`input/mod.rs`) | Collision / note |
|---|---|---|---|---|
| `Escape` | `0x01` | single | `SCAN_ESC = 1` | **Do NOT forward `KeyCode::Escape` as `0x01`**: `from_byte(0x01)` decodes to `('a', ctrl=true)` — the Ctrl+A binding (`ToggleAot`, a desktop grab) — so hardware Esc would toggle always-on-top instead of dismissing. `from_byte` deliberately does not decode scan 1; Esc already flows as Unicode `0x1B` (`us104.rs:23`), so leave it out of the RawKey forward set |
| `Enter` | `0x1C` | single | `SCAN_ENTER = 28` | delivered today as Unicode `0x0A` (`us104.rs:317`); `from_byte` accepts `0x0A`/`0x0D`/`28` |
| `Backspace` | `0x0E` | single | (none — `KEY_BACKSPACE`=0x7F / `KEY_BACKSPACE_ALT`=0x08) | delivered today as Unicode `0x08` (`us104.rs:108`); the set-1 `0x0E` is not a userspace name |
| `ArrowUp` | `0x48` | **E0-extended** | `SCAN_UP = 72` | single-byte `0x48` = Numpad8 |
| `ArrowLeft` | `0x4B` | **E0-extended** | `SCAN_LEFT = 75` | single-byte `0x4B` = Numpad4 |
| `ArrowRight` | `0x4D` | **E0-extended** | `SCAN_RIGHT = 77` | single-byte `0x4D` = Numpad6 |
| `ArrowDown` | `0x50` | **E0-extended** | `SCAN_DOWN = 80` | single-byte `0x50` = Numpad2 |
| `F1`…`F10` | `0x3B`…`0x44` | single | (none yet) | |
| `F11` | `0x57` | single | `SCAN_F11 = 0x57` | `0x57` = ASCII `'W'` in a byte stream (indistinguishable — the constant's comment notes the legacy overlay toggled on both) |
| `F12` | `0x58` | single | (none) | |
| `Home`/`PageUp`/`End`/`PageDown`/`Insert`/`Delete` | `0x47`/`0x49`/`0x4F`/`0x51`/`0x52`/`0x53` | E0-extended | (none) | same E0-second-byte convention |
| `NumpadEnter`/`ControlRight`/`AltRight`/`NumpadSlash` | `0x1C`/`0x1D`/`0x38`/`0x35` | E0-extended | (none) | R-Ctrl/R-Alt second bytes — the modifier tracker's `0xE0` no-op arm plus the standalone `0x1D`/`0x38` arms already catch these |

**Spec-only un-gate (kernel is mid-rewrite):** at `gui/window.rs:209`,
replace the drop with a match that pushes the packed value (Design A:
`byte | (alt<<8) | (ctrl<<9) | (shift<<10)`, mods from the compositor's
tracker), forwarding the arrows + F11/F12 first and leaving the rest
dropped — **Escape stays dropped** (see the table row: `0x01` would decode
as Ctrl+A). Userspace needs no change: `from_byte` decodes `72/75/77/80`
to the `SCAN_*` codes the a11y ring already matches (`desktop.rs` a11y
arms), and `SCAN_F11` toggles the debug overlay (safe only because the
keymap row for `0x57` exists — it also catches ASCII `'W'`, pre-existing).
One caveat: an arrow carrying any modifier bit (Shift+arrow, etc.) skips
the a11y guard (`key & 0xFF00 != 0 → false`) and has no keymap row, so it
is inert until rows are added — correct today (arrows do not arrive at
all), but shifted-arrow navigation needs keymap rows later. The userspace
constant values are pinned by
`tests/test_login_flow.py::TestScancodeConstants`.

---

## 3. Feature 1 — Phase B Tab/Enter

**Evidence says the byte path already works today:** Tab (0x09) and Enter
(0x0A) are `Unicode` control chars (not RawKey), so they reach `key_events`
(`window.rs:207`) and `sys_gui_get_key` unmodified; `login-manager` already
handles `0x09` (Tab → field switch) and `0x0A|0x0D` (Enter → submit)
(`login-manager/src/main.rs`). A modifier byte is **not** required for this
feature.

The remaining Phase B gate is therefore empirical, and has a CI-greppable
probe already in the kernel: `[KBD] IRQ1 fired!` (`interrupts.rs:655`) proves
IRQ1 actually fires and scancodes reach `handle_scancode`. The kernel rewrite
should first verify that marker appears on the GUI boot; if it does not, the
blocker is IRQ1 routing / PS/2 init (`drivers/ps2.rs:108-123` enables the
controller: `0xAE` enable, `0xFF` reset, `0xF4` enable scanning) or
`gui_refresh_task` starvation, not the decode path.

Closed on real input (Aug 12, 2026): `qemu_gui_gate.exp` step 4a sends
`sendkey tab` and asserts login-manager's serial announce
`[login] tab: focus -> password` (its Tab arm, `login-manager/src/main.rs`) —
proving a Unicode key from the monitor reaches the GUI consumer and drives
the field advance, not just IRQ1 delivery.

If the rewrite wants Tab/Enter indistinguishable from the set-1 scan codes the
userspace keymap also names (`SCAN_ENTER = 28`), that is a separate
RawKey→scancode mapping decision; today the Unicode bytes are sufficient.

---

## 4. Feature 2 — Ctrl+Alt+Backspace chord (genuinely blocked)

The chord needs ctrl+alt to reach the keymap; the current path cannot carry
it:

- `window.rs:26` `key_events: VecDeque<u8>` — one byte per key.
- `window.rs:207` pushes only `c as u8`.
- `sys_gui_get_key` (`syscalls/mod.rs:4792`) returns one byte.
- `libsarga get_key` (`gui.rs:464`) returns `Option<u8>`; `input/mod.rs
  from_byte` forces `alt: false, shift: false`.

Backspace itself arrives as 0x08 (which userspace already normalizes to
`KEY_BACKSPACE` = 0x7F), so the chord needs **only the two modifier bits** —
no key-code change.

### The mechanism (verbatim-able)

**A. `HandleControl::MapLettersToUnicode`** at `main.rs:428`.
Effect: Ctrl+letter decodes to U+0001..U+001A — exactly the control-code range
`from_byte` already maps back to Ctrl+letter. Unblocks every Ctrl+letter
shortcut (Ctrl+W close, Ctrl+S settings, GUI-terminal Ctrl+C — which today
under `Ignore` is a plain `c` typed into the terminal). Side-effect check: the
ctrl branch applies only to letters (`us104.rs:221+`), so Alt+Tab,
Super+arrows, Tab, Enter, Backspace, F-keys are untouched.

**B. Modifier byte through the chain** (bit0=ctrl, bit1=alt, bit2=shift):

- `window.rs:26` — `key_events: VecDeque<u32>` packed as `byte | (mods << 8)`
  (0 stays the empty sentinel).
- `main.rs gui_refresh_task` — track ctrl/shift from raw set-1 scancodes
  exactly like Alt/Super today (lines 432-445): `0x1D/0x9D` L-Ctrl,
  `0xE0 0x1D/0x9D` R-Ctrl, `0x2A/0xAA` L-Shift, `0x36/0xB6` R-Shift; Alt
  `0x38/0xB8` already tracked. (Recommended over reading pc_keyboard's
  `modifiers`, which is a **private** field — `lib.rs:52`; the raw-scancode
  tracking already exists and works.) Pass `mods` into
  `comp.handle_keyboard(key, mods)`.

  **Required sub-change, not a given:** the existing `0xE0` arm (line 435)
  is a **no-op stub** ("next byte is the real scancode" — it does nothing),
  and the `0x5B/0xDB` Win tracking ignores the prefix entirely. R-Ctrl/R-Shift
  tracking (`0xE0 0x1D/0x9D`, `0xE0 0x36/0xB6` in set 1) therefore requires
  implementing real E0 state handling. Without it, R-Ctrl/R-Shift would
  mis-decode as L-Ctrl/L-Shift.

  **Authoritative-source decision:** the compositor's own chords (Alt+Tab
  cycle, Super+snap, Alt+F4-close) currently key off `alt_held`/`super_held`
  from this raw tracking. Once a mods byte flows to windows, declare the mods
  byte authoritative for window delivery and migrate the chords to it (or
  prove they can never diverge) — otherwise a scancode-drop desync between
  the two sources could swallow a chord (e.g. Alt+Tab's "confirm on alt
  release" path eating Ctrl+Alt+Backspace).

  (`Keyboard.modifiers` is the private field at `lib.rs:52`; the
  `Modifiers` struct fields themselves are public at `lib.rs:240-247`.)
- `gui/mod.rs:571` — `handle_keyboard(key: DecodedKey, mods: u8)`; forward to
  `window.handle_keyboard(key, mods)` (both arms).
- `window.rs:207` — `push_back((c as u8 as u32) | ((mods as u32) << 8))`.
- `syscalls/mod.rs:4792` — return the packed u32; 0 = empty (unchanged
  contract).
- `libsarga/src/gui.rs:464` — `get_key` returns `Option<u16>` (byte |
  mods<<8); `sys_gui_get_key` is GUI-only (the console getty reads
  `/dev/tty0` from `TTY_INPUT` by a different path), so there is **no other
  consumer of this API** — change its shape freely. The packed value maxes at
  `0x7E | 0x07<<8 = 0x77E`, safely inside u16.
- `ade/src/input/mod.rs` — `from_byte` gains a mods-aware variant (or a
  `KeyEvent` setter) so `alt`/`shift` flow into `resolve`; the chord
  (0x08 → normalized 0x7F + ctrl|alt) resolves to `Quit`, and the pinned
  near-miss matrix (`test_session_end_gate`) starts distinguishing
  Ctrl+Backspace / Alt+Backspace / Ctrl+Alt+Shift+Backspace in the kernel
  instead of only synthetically.

Expected decode, end to end: Ctrl+Alt+Backspace → IRQ1 → scancodes
`0x1D 0x38 0x0E` (ctrl, alt, backspace make) → `handle_keyboard(Unicode(0x08),
0x03)` → key_events `0x0803` → `sys_gui_get_key` → libsarga → `KeyEvent(0x7F,
ctrl=true, alt=true, shift=false)` → `resolve` → `Quit`. Exact-match table row
(`input/mod.rs` BINDINGS) — matches all three bits, so near-misses stay no-ops.

---

## 5. Verification plan for the rewrite

1. **IRQ routing gate** — grep serial log for `[KBD] IRQ1 fired!` on the GUI
   boot; add it to the GUI-reachability job if not already gated.
2. **Phase B** — `qemu_gui_login.exp`: `sendkey tab`, `sendkey ret`, letters;
   assert `[login] window created` and the session markers. If Tab/Enter still
   fail with the marker present, the bug is downstream of IRQ1 and the log
   shows it.
3. **Chord** — after B lands, drive Ctrl+Alt+Backspace via the QEMU monitor's
   **hyphenated chord form** `sendkey ctrl-alt-backspace` (QEMU presses the
   whole chord); assert `[ade] session ended` + init respawn (the exp already
   greps those markers). Do **not** use the exp harness's existing
   per-key `sendkey_seq` helper — it presses *and releases* each key, so ctrl
   and alt would be up before backspace is pressed and the mods byte would
   never read ctrl+alt.
4. **Userspace pins** — `ade` selftests `test_keymap` (binding dump: count 18,
   chord present, Ctrl+Q absent) and `test_session_end_gate` (near-miss
   matrix) are the host-verifiable contract; once libsarga delivers mods,
   `test_keymap` extends `from_byte` coverage from synthetic to decoded mods.

---

## 6. Assumptions and open questions

- **Assumption:** the kernel rewrite keeps pc-keyboard 0.5.1 (variant name is
  `MapLettersToUnicode`; the user-facing name `MapLettersToControl` belongs to
  older versions — grep `HandleControl` in `Cargo.lock` before porting).
- **Open:** whether QEMU monitor `sendkey` scancodes reach IRQ1 on this build
  (`[KBD] IRQ1 fired!` is the probe; unverified here — no QEMU run was made).
- **Open:** pc_keyboard `Keyboard::modifiers` is private (`lib.rs:52`); the
  doc recommends raw-scancode tracking (already in place for Alt/Super) rather
  than patching the crate.
- **Spec'd (Aug 10, 2026):** arrows/F-keys stay RawKey-dropped
  (`window.rs:202,209`) on today's kernel; the `KeyCode` → set-1 scancode
  mapping that un-gates them is now tabulated in §2.1 (arrows are the E0
  extended second bytes — `0x48`/`0x4B`/`0x4D`/`0x50` — not the single-byte
  numpad codes), with the spec-only `gui/window.rs:209` change sketched and
  the userspace constant values pinned by
  `tests/test_login_flow.py::TestScancodeConstants`.
- **Userspace ripple:** `get_key`'s return type and `from_byte`'s modifier
  handling must land with the kernel change or the chord stays synthetic-only.
