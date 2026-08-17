# Kernel termios: ICANON line editing for the console tty

**Status:** SPEC ONLY — the kernel is under external major change; not applied.
**Date:** Aug 11, 2026. **Verified against:** live kernel tree
(`SKYIOUS KERNEL/kernel/src/`), login/sash/login-manager userspace
(`SkyOS/login`, `SkyOS/sash`, `SkyOS/login-manager`).
**Companion:** builds directly on `kernel-tcsets-echo.md` (the `TTY_TERMIOS`
global and honored `ECHO`). Landing order: the ECHO spec first, or both
together — the discipline here reads `TTY_TERMIOS` for `ICANON`/`ECHO`.
Same convention: the kernel rewrite picks this up verbatim.

---

## 1. The gap

`c_lflag` defaults to `0xB` (`ISIG|ICANON|ECHO`, per kernel-tcsets-echo.md
§3.1) — **ICANON is advertised but not implemented.** The console read path
(`vfs/devfs.rs:55-66`) is a raw byte pop of `tty::TTY_INPUT` with no line
buffer, no erase, and no Enter handling beyond the input-time `\n`→`\r`
conversion (`tty.rs:48-49`). Consequence at the console getty: Backspace is a
literal `0x7F` byte embedded in the username/password — the current console
login **cannot erase a mistype** (verified: `login/src/main.rs:114-127`
`read_line` has no `0x7F`/`0x08` arm).

Every console reader hand-rolls what a line discipline should own:

| Reader | Path | Terminates on | Backspace | Self-echo |
|---|---|---|---|---|
| console getty (`login/src/main.rs:114-127` `read_line`) | fd 0 → devfs Tty0 | `\n`/`\r` | **none** | no |
| console shell raw (`sash/src/main.rs:567-586` `read_raw_line`) | fd 0 | `\n`/`\r` | yes (`0x7F`/`0x08` → `pop`) | no |
| console shell editor (`sash/src/readline.rs:788` `read_line`, used at `main.rs:393`) | fd 0, raw `syscall3(0, buf, 4096)` | Enter | yes (`readline.rs:914`) | **yes** (`redraw_with_mode`) |
| GUI login (`login-manager/src/main.rs:124`) | `SYS_GUI_GET_KEY` (not the tty) | Enter | yes (`0x7F`/`0x08`) | GUI-drawn |

Two load-bearing facts this spec must respect:

1. **The console tty read never blocks** (`devfs.rs:37-66`, unchanged from the
   committed build). An empty `TTY_INPUT` makes `read()` return `0`.
   `login` treats `0` as EOF (`read_line` → `Ok(None)` → `exit(1)`,
   `main.rs:217-218`); sash's editor treats `n <= 0` as EOF and breaks its
   loop (`readline.rs:802`). A working getty therefore requires **blocking
   tty reads** — see D0. The historical getty behavior (re-prompt loop,
   probe login reaching sash) implies the working build blocked; the current
   tree does not. This is an unresolved transition D0 settles.
2. **sash's interactive editor needs raw per-keystroke input and self-echoes.**
   Naive kernel-side ICANON (default on) would hand it whole pre-edited lines
   on Enter only — no per-key navigation, no history, no completion — and
   kernel ECHO would double-echo its `redraw`. The spec must therefore
   include a userspace companion: sash clears `ICANON|ECHO` (raw) while
   running its editor, exactly as real `sh` does with termios.

---

## 2. D0 — required foundation: blocking tty reads

Before any line discipline: a console `read()` with no data available must
**block (yield) until bytes arrive, and return `0` only at real EOF**
(fd closed / peer gone). Rationale: every current console reader keys its
EOF handling off `0`; an empty queue is *not* EOF. (The pty has the same
shape today — `pty_read_slave` returns `Ok(0)` on an open-but-empty slave —
but its consumer polls, so it tolerates it; login and sash do not.)

Concrete change, in the devfs Tty0 read arm (`devfs.rs:55-66`): replace the
pop-until-empty loop with

```rust
// Tty0 read arm — blocking, one line per call in canonical mode.
DevNodeInner::Tty0 => {
    let line = crate::tty::read_tty(max_len);   // blocks via scheduler yield
    Ok(line)
}
```

with `read_tty` defined in `tty.rs` (next to `TTY_INPUT`): it yields
(`crate::task::scheduler::yield_now()` or the kernel's established
park/wake primitive) while no data is available, and returns `0` only when
the underlying fd is closed. This one change is **independently required** —
it fixes login exiting at the prompt and sash's editor dying on empty reads
even without ICANON.

> Note for the rewrite: the O_NONBLOCK path (return `EAGAIN` instead of
> blocking) is out of scope here; no current console reader sets it.

---

## 3. Design A (recommended): input-side line discipline

Editing and echo live in the **input path** (`tty.rs feed_scancode`), the
same IRQ context that already injects the `^C` echo (`tty.rs:31-33`). The
read side just pops one complete line. This matches how a real tty line
discipline works and keeps echo at input time, where the edited character
is actually visible.

### Change A1 — `kernel/src/tty.rs`: canonical line queue + discipline

> **IRQ-context invariant:** `feed_scancode` runs from the keyboard IRQ.
> AGENTS.md forbids allocations in IRQ paths ("never add allocations to IRQ
> paths"; the allocator lock is held across the poison loop) — which is why
> `TTY_INPUT` is a preallocated `ArrayQueue`. The canonical structures below
> follow the same rule: **fixed-size line slots, preallocated at boot, zero
> runtime allocation.** A pasted line longer than `LINE_MAX` is truncated
> (line dropped, not grown); a full `TTY_LINES` drops the newest line.

```rust
/// Max bytes per canonical line (matches the devfs read cap `min(max_len, 256)`).
const LINE_MAX: usize = 256;
/// Number of complete lines buffered between the IRQ and the reader.
const LINE_SLOTS: usize = 8;

lazy_static! {
    // ...existing TTY_INPUT...
    /// Complete lines from the canonical editor (ICANON mode). Each slot is
    /// a fixed-size line INCLUDING its terminator ('\r' by default; '\n' if
    /// ICRNL is ever set). Preallocated — push/pop never allocate.
    /// Raw mode (ICANON clear) keeps using TTY_INPUT byte-for-byte as today.
    pub static ref TTY_LINES: ArrayQueue<([u8; LINE_MAX], usize)> =
        ArrayQueue::new(LINE_SLOTS);
    /// Current partial line while ICANON is active. Fixed-size + len, so the
    /// erase/append path in IRQ context never allocates.
    pub static ref TTY_CANON_BUF: Mutex<([u8; LINE_MAX], usize)> =
        Mutex::new(([0; LINE_MAX], 0));
}
```

Rewrite the `DecodedKey::Unicode(c)` arm of `feed_scancode`:

```rust
DecodedKey::Unicode(c) => {
    if c == '\u{3}' {
        // Ctrl+C — VINTR: SIGINT to the foreground process, discard the
        // pending canonical line (ISIG), echo "^C\r" to the console ONLY
        // (see A5 — today this leaks a fake "^C" line into TTY_INPUT).
        let proc = crate::task::process::CURRENT_PROCESS.lock();
        if let Some(ref p) = *proc {
            p.signals.lock().raise(crate::syscalls::signal::Signal::SIGINT);
        }
        crate::tty::tty_echo(b"^C\r");
        *crate::tty::TTY_CANON_BUF.lock() = ([0; LINE_MAX], 0); // discard line
        return;
    }
    let lflag = crate::tty::TTY_TERMIOS.lock().c_lflag;
    let icanon = lflag & 0x2 != 0;          // ICANON
    let echo = lflag & 0x8 != 0;            // ECHO
    if icanon {
        // Take the partial line OUT of the guard first: no lock is held
        // across tty_echo or the TTY_LINES push (same drop-the-guard
        // discipline as the companion's ECHO path, and shorter IRQ hold).
        let mut line = core::mem::take(&mut *crate::tty::TTY_CANON_BUF.lock());
        let (buf, len) = (&mut line.0, &mut line.1);
        match c as u8 {
            b'\x08' | b'\x7F' => {          // BS/DEL: erase last char
                if *len > 0 { *len -= 1; if echo { crate::tty::tty_echo(b"\x08 \x08"); } }
            }
            b'\r' | b'\n' => {              // line terminator (input path
                                            // converts '\n'->'\r', so this
                                            // is almost always '\r')
                if echo { crate::tty::tty_echo(b"\r"); }
                buf[*len] = b'\r';          // terminator INCLUDED (see §4)
                let done = (*buf, *len + 1);
                // Push into the preallocated slot queue; drop-new on full
                // (IRQ context: never allocate, never block).
                let _ = crate::tty::TTY_LINES.push(done);
                crate::tty::wake_tty_reader();
                *crate::tty::TTY_CANON_BUF.lock() = ([0; LINE_MAX], 0);
                return;
            }
            _ => {
                if *len < LINE_MAX {
                    if echo { crate::tty::tty_echo(&[c as u8]); }
                    buf[*len] = c as u8;
                    *len += 1;
                } // else: line full, drop the char
            }
        }
        *crate::tty::TTY_CANON_BUF.lock() = line;
    } else {
        if c == '\n' { let _ = TTY_INPUT.push(b'\r'); }
        let _ = TTY_INPUT.push(c as u8);
    }
}
```

`tty_echo(bytes)` is the console+serial emit extracted from the companion's
ECHO path (`WRITER.lock().write_byte` + `serial_putc` per byte). It must be
callable from IRQ context — the existing `^C` path already runs there, so
the WRITER/serial locks are the same ones the write path takes.

### Change A2 — `kernel/src/tty.rs`: blocking read helper

```rust
/// Console read. Canonical mode: return the oldest complete line (blocking,
/// D0) — never a partial line. Raw mode: block for the first byte, then
/// return what is immediately available. Returns 0 only at real EOF (the
/// tty fd closed — never on an empty queue; login and sash key EOF off 0).
pub fn read_tty(max_len: usize) -> Vec<u8> {
    let lflag = TTY_TERMIOS.lock().c_lflag;
    if lflag & 0x2 != 0 {
        loop {
            if let Some((buf, len)) = TTY_LINES.pop() {
                let n = core::cmp::min(len, max_len);
                return buf[..n].to_vec(); // one complete line, never partial
            }
            // D0: yield until a line lands. wake_tty_reader() (A1) must be
            // an interrupt-safe flag/condvar — the IRQ pushes and clears a
            // flag; this loop re-checks after each yield. No lock is taken
            // in IRQ context (see the A1 invariant). Plus a closed-tty
            // check that returns Vec::new() at EOF.
            crate::task::scheduler::yield_now();
        }
    }
    // Raw mode: block until at least one byte, then pop up to max_len.
    let mut out = Vec::new();
    loop {
        if let Some(c) = TTY_INPUT.pop() { out.push(c); if out.len() >= max_len { break; } }
        else if out.is_empty() { crate::task::scheduler::yield_now(); }
        else { break; }
    }
    out
}
```

### Change A3 — `kernel/src/vfs/devfs.rs`: canonical read arm

Replace the Tty0 arm (`:55-66`) with `Ok(crate::tty::read_tty(max_len))`.
The read side is now trivial; all editing/echo lives in A1/A2.

### Change A4 — the companion ECHO spec is superseded for canonical mode

kernel-tcsets-echo.md §3.3 echoes in the **read path**. With canonical mode
that is wrong — the character is edited (or erased) before any read, so echo
must happen at input time (A1). When ICANON lands:

- Canonical mode: echo only from `feed_scancode` (A1). The §3.3 read-side
  echo must be skipped (`&& c_lflag & ICANON == 0`), or removed once raw
  mode is the only remaining user.
- Raw mode: §3.3's read-side echo remains the implementation.

### Change A5 — stop injecting `^C\r` into `TTY_INPUT`

Today `tty.rs:31-33` pushes `^`, `C`, `\r` into `TTY_INPUT`, so the console
reader receives a fake `"^C"` line (login would treat it as a username;
sash would execute `^C`). A1's Ctrl+C arm replaces this with a console-only
`tty_echo(b"^C\r")` and discards the pending canonical line (POSIX VINTR
discards the line unless `NOFLSH`).

---

## 4. The interaction with existing readers (the point of this spec)

| Reader | With kernel ICANON (A1-A3 + D0) | Userspace change needed |
|---|---|---|
| console getty `login read_line` | **works unchanged.** Discipline delivers `"line\r"` (terminator INCLUDED — critical: login submits on `\r`; if the terminator were stripped, login would block forever). Its own backspace gap is fixed for free. Its lack of `0x7F` handling is now moot (the byte never arrives). | none |
| console shell `sash read_raw_line` | works unchanged (submits on `\r`; its own `0x7F`→`pop` becomes dead-but-harmless). | none |
| console shell `sash readline` (editor) | **degrades to line-at-a-time if left in canonical mode**: it still executes commands (it gets one pre-edited line per Enter) but loses cursor/history/completion, and arrow-key escapes (`0x1B [ A`) would be swallowed into the line. Kernel ECHO + its own `redraw` = double echo. | **must clear `ICANON|ECHO` (raw) via TCSETS at editor start**, the standard shell/readline pattern (cf. `termios` raw mode in real `sh`). Optional, documented companion change in `sash/src/readline.rs`. |
| GUI `login-manager` password field | unaffected — GUI key pipeline (`SYS_GUI_GET_KEY`), not the tty. | none |
| ade terminal (pty) | unaffected — separate path; the pty already has its own `PtyLineDiscipline { echo, canonical }` (`pty.rs:26-35`) with a read-side canonical arm (`pty.rs:100-121`). Two disciplines exist; keep their semantics aligned (both deliver terminator-included lines). | none |

Also inherited from the ECHO spec: **no reader self-echoes except sash's
editor** — login and `read_raw_line` only read. So kernel ECHO is the single
echo source for the getty and the raw console shell; the sash editor clears
ECHO (above). The companion's §5 claim ("the only TTY_INPUT reader is the
console getty/login") is wrong once sash runs — this spec corrects it.

Byte-stream equivalence: with `c_iflag=0` (no ICRNL, the current default),
the canonical discipline delivers `"line\r"` — byte-identical to what login
reassembles today, plus backspace now actually erases. `tests/test_login_flow.py`
(re-prompt pins, attempt-cap pins, `Ok(None)`-is-EOF pins) stays valid.

---

## 5. Design B (minimal): read-side editor mirroring the pty

Reuse the pty's canonical read shape (`pty.rs:100-121`) for the console:
a persistent `TTY_CANON_BUF` drained from `TTY_INPUT` at read time, erase on
`0x7F`/`0x08`, return on CR/NL.

| | A (input-side) | B (read-side) |
|---|---|---|
| Echo | input time (correct for canonical) | read time — misses erase echo until the next read; same problem the companion has |
| Blocks | D0 everywhere | still needs D0 |
| Matches in-tree precedent | `^C` echo (`tty.rs:31`) | pty canonical arm (`pty.rs:100`) |
| Lines delivered mid-keystroke | reader sees nothing until Enter | reader may see partial bytes if it reads before Enter (breaks line atomicity) |
| Kernel lines | ~50 across tty.rs + devfs one-liner | ~30 in devfs/tty read path |

**Recommendation: A.** It is the only shape where echo and erase happen at
input time, and it keeps the read side line-atomic. B is the fallback if the
rewrite prefers one read-site change over touching `feed_scancode`.

---

## 6. Verification plan (once applied)

1. **Build:** kernel release; workspace clippy `-D warnings`.
2. **Kernel selftest** (mirroring the ECHO/ICANON pattern): a
   `tty::selftest_canonical` that feeds bytes through the discipline
   (type `roo` + `0x7F` + `t\r`) and asserts `TTY_LINES` holds `"root\r"`;
   asserts the erase sequence `\x08 \x08` is echoed only when `ECHO`; and a
   raw-mode case asserting the byte stream is unchanged.
3. **Host gates:** `tests/test_login_flow.py` must stay green untouched
   (proves the delivered byte stream is compatible); the attempt-cap /
   re-prompt / EOF pins are the regression net.
4. **QEMU (needs a bootable kernel — the current tree does not boot, the
   reason this is a spec):**
   - getty backspace: type `roo` + Backspace + `t\r` at `login:`, assert the
     username submits as `root` and logs in — a new positive assertion in
     `qemu_shell_test.exp`'s login leg.
   - blocking: `login:` prompt persists with no respawn flicker while idle
     (the existing no-respawn greps cover it) — proves D0.
   - sash editor: with the §4 companion change, arrows/history still work in
     the console shell (raw mode); without it, the harness would observe the
     editor hang — the companion change is mandatory.
5. **Cross-spec:** kernel-tcsets-echo.md's §5 correction (sash is also a
   `TTY_INPUT` reader) and §3.3's ECHO-placement note (input-side for
   canonical) get one-line updates.

## 7. Assumptions / open questions

- **Kernel mid-major-change:** anchors verified Aug 11, 2026 against the
  dirty tree; `feed_scancode`, `read_tty`, `TTY_LINES` are new names, line
  numbers may drift.
- **D0's yield primitive:** the spec assumes a `scheduler::yield_now()`
  exists or is trivially added; the rewrite should use its established
  park/wake mechanism. The closed-tty EOF check must live in `read_tty` so
  `0` remains reserved for EOF.
- **IRQ-context allocation (A1):** the canonical editor runs in the keyboard
  IRQ, so `TTY_LINES`/`TTY_CANON_BUF` must stay preallocated and bounded as
  spec'd (line truncation + drop-new on overflow are intentional, matching
  the `ArrayQueue` discipline of `TTY_INPUT`). If the rewrite ever moves
  input processing out of IRQ context (deferred workqueue), the bounded
  structures can be relaxed.
- **Terminator included:** canonical lines keep their `\r`. Stripping it
  would deadlock login (submit key is `\r`) — do not "clean up" the
  terminator to look POSIX without also changing login.
- **ICRNL is off by default** (`c_iflag=0`): if the rewrite enables ICRNL,
  the discipline must deliver `\n` terminators and echo `\r\n`; login and
  `read_raw_line` accept both today, but the pty discipline (`pty.rs:106-108`)
  already normalizes to `\n` — keep the two consistent.
- **sash companion change is mandatory, not optional:** default-on ICANON
  (0xB) without sash clearing ICANON/ECHO degrades the console shell editor
  and double-echoes it. The kernel spec ships with this documented.
- **login sets ECHO explicitly for the username read (userspace, Aug 12,
  2026):** `login/src/main.rs::ensure_echo(fd)` (`TCGETS` → OR ECHO →
  `TCSETS`) now runs before the `login: ` prompt, because the username read
  happens before `echo_off`. With A1's input-time echo this is reinforcing,
  not conflicting: ECHO is guaranteed set, so the username echoes at input
  time in canonical mode exactly as a getty expects, and the byte stream to
  `read_line` is unchanged (`"line\r"`). No interaction-table change.
- **VINTR line-discard:** A5 discards the canonical buffer on Ctrl+C; POSIX
  `NOFLSH` (don't discard) is out of scope — no consumer needs it.
- **Open:** should raw-mode reads (D0) also block, or is the getty the only
  consumer? This spec blocks both for uniformity; the rewrite may prefer
  blocking only in canonical mode if a poller ever appears.
