# Kernel termios: honor the ECHO bit via TCSETS

**Status:** SPEC ONLY — the kernel is under external major change; not applied.
**Date:** Aug 9, 2026. **Verified against:** live kernel tree
(`SKYIOUS KERNEL/kernel/src/`), login userspace, vendored libsarga.
**Intended for:** the kernel rewrite to pick up verbatim (same convention as
`kernel-gui-modifier-delivery.md`).

---

## 1. The gap

Userspace already does the full getty/login dance forward-compatibly:

- `login/src/main.rs` `echo_off(fd)` — `TCGETS` → clear `c_lflag & !ECHO` →
  `TCSETS`; `echo_on(fd, lflag)` restores. `read_password` wraps
  `read_line` with echo_off/echo_on.
- The termios layout is mirrored in userspace (`repr(C)`, 4 `u32` fields +
  `c_cc: [u8; 19]`), and `ECHO = 0x8` per POSIX is pinned in login.
- libsarga `ioctl()` forwards `TCGETS`/`TCSETS` (0x5401/0x5402).

But the kernel does **not** honor it:

- `kernel/src/syscalls/mod.rs:2350-2365` — `TCGETS` returns a *hardcoded*
  `Termios` (`c_lflag: 0x5`, comment claims "ICANON | ECHO", but POSIX
  0x5 = ISIG|ICANON; ECHO 0x8 is NOT set); `TCSETS` is a **no-op returning 0**.
- `kernel/src/vfs/devfs.rs:55-66` — the Tty0 read path pops
  `tty::TTY_INPUT` with **no echo**; there is no termios state anywhere in
  the kernel (`grep termios kernel/src` → only sys_ioctl).

Consequence: login's `echo_off` silently no-ops. If echo-on-read is ever
added to the kernel, the password would be echoed regardless of login's
TCSETS call. This spec wires the state so the forward-compat path becomes
real: **TCSETS stores, TCGETS returns, and the read path echoes only when
ECHO is set.**

## 2. Design

Termios must be **one shared instance** for the tty device: both the
`tty0` and `tty` devfs nodes carry `DevNodeInner::Tty0` (unit variant,
`devfs.rs:15`, constructed at `:335/:340`), so per-node state would let the
two aliases diverge. Mirror the existing `TTY_INPUT` global
(`kernel/src/tty.rs:12`): a `TTY_TERMIOS: Mutex<Termios>` global in
`tty.rs`. `sys_ioctl`'s TCGETS/TCSETS arms (which already run before any
fd→node resolution and are the single termios surface) read/write it; the
devfs Tty0 read path consults it for the ECHO bit.

> **Deviation from the request's wording ("store termios state on the
tty0 node").** A per-node field (e.g. `Tty0(Mutex<Termios>)`) is *also*
correct for a single tty, but (a) `sys_ioctl` handles TCGETS/TCSETS before
the fd→node dispatch, so a global is reachable without resolving the node
at all, and (b) the getty's fd 0 may be `/dev/tty0` *or* `/dev/tty` — a
global works regardless of which alias the caller holds, while per-node
state would silently depend on the caller using the same node for TCSETS
and read. The global is the minimal, alias-proof choice; it moves into a
per-tty struct the day `TTY_INPUT` does (see §5).

The default `c_lflag` must gain `ECHO` (0x8): a fresh boot should echo
typed input (normal interactive behavior), and login's `echo_off` then
actually suppresses it during password entry. POSIX values: `ISIG=0x1`,
`ICANON=0x2`, `ECHO=0x8`.

## 3. Exact changes

### 3.1 `kernel/src/tty.rs` — add the termios global

Add after the existing `TTY_INPUT` lazy_static:

```rust
#[repr(C)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_cc: [u8; 19],
}

impl Termios {
    /// Defaults: raw-iflag/oflag, CLOCAL|CREAD|CS8, ISIG|ICANON|ECHO.
    pub fn default_tty() -> Self {
        Termios {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0xBF, // CLOCAL | CREAD | CS8
            c_lflag: 0xB,  // ISIG(0x1) | ICANON(0x2) | ECHO(0x8)
            c_cc: [0; 19],
        }
    }
}
```

and inside the `lazy_static!` block:

```rust
pub static ref TTY_TERMIOS: Mutex<Termios> = Mutex::new(Termios::default_tty());
```

(Place the `Termios` struct and `impl` above the `lazy_static!` — the
`default_tty()` call must be nameable inside it.)

### 3.2 `kernel/src/syscalls/mod.rs` — TCSETS stores, TCGETS returns

Replace the current TCGETS/TCSETS arms (`:2350-2365`):

```rust
        TCGETS => {
            let t = crate::tty::TTY_TERMIOS.lock().clone();
            if unsafe { user_access::copy_to_user(argp, core::slice::from_raw_parts(
                &t as *const _ as *const u8, core::mem::size_of::<crate::tty::Termios>(),
            )) }.is_err() {
                return errno::Errno::EFAULT as u64;
            }
            0
        }
        TCSETS => {
            let mut t = crate::tty::TTY_TERMIOS.lock();
            let dst = core::slice::from_raw_parts_mut(
                &mut *t as *mut _ as *mut u8,
                core::mem::size_of::<crate::tty::Termios>(),
            );
            if unsafe { user_access::copy_from_user(dst, argp) }.is_err() {
                return errno::Errno::EFAULT as u64;
            }
            0
        }
```

Notes:
- The local `Termios` struct inside `sys_ioctl` (`:2323-2330`) can be
  deleted; the shared `crate::tty::Termios` is the single definition.
  **Before deleting, diff the two structs** (field order, `c_cc` size,
  `repr`) — userspace mirrors the layout as `repr(C)` 4×`u32` + `c_cc:
  [u8; 19]`, so an unnoticed mismatch would be a silent ABI break.

  **Audit Aug 12, 2026 — no drift, verified (two mirrors since the passwd
  rewire):** the userspace mirrors are `login/src/main.rs` (`#[repr(C)]`,
  `c_iflag`/`c_oflag`/`c_cflag`/`c_lflag: u32` + `c_cc: [u8; 19]`) and
  `passwd/src/main.rs` (same layout, added with the passwd echo rewire),
  both byte-identical to the kernel struct here and to each other. libsarga
  defines no `Termios` (its `ioctls` module carries only the TCGETS/TCSETS
  request numbers); sash, coreutils, login-manager, init, svc, vahid define
  none. Byte contract: 4×u32 + 19 = 35 data bytes; `repr(C)` pads `size_of`
  to 36 (align 4) — the TCSETS store must copy exactly
  `size_of::<Termios>()` from the caller's buffer. Pinned host-side by
  `tests/test_login_echo.py::test_termios_layout_size_contract`,
  `::test_passwd_termios_mirrors_login_layout` (login vs passwd byte
  equality), and `::test_single_userspace_termios_definition` (the latter
  scans the workspace and fails if any third userspace definition appears).
- `Termios` derives `Clone` so `TCGETS` can copy out of the lock without
  holding it across the `copy_to_user`.
- **Lock-hold asymmetry (intentional):** TCSETS holds the guard across
  `copy_from_user` while TCGETS clones-then-copies-out. Both are sound
  (single `&mut [u8]` derived from the guard; the guard drops on the
  EFAULT return). The hold is bounded — a ~36-byte copy — and
  `TTY_TERMIOS` is only contended with the read-arm echo check, so it is
  not a stall hazard. If the maintainer prefers symmetry, copy into a
  stack local first, then store it in the mutex.
- `copy_from_user` takes `(dst: &mut [u8], src_ptr: *const u8)`
  (`kernel/src/syscalls/user_access.rs:141`); the TCSETS arm builds the
  destination slice from the locked termios as above — this matches the
  established pattern (e.g. `syscalls/mod.rs:1257`).

### 3.3 `kernel/src/vfs/devfs.rs` — Tty0 read path echoes only with ECHO

Replace the Tty0 read arm (`:55-66`) with:

```rust
            DevNodeInner::Tty0 => {
                let n = core::cmp::min(max_len, 256);
                let mut buf = Vec::with_capacity(n);
                // Echo flag computed once; the guard drops at the end of this
                // statement, so TTY_TERMIOS is never held across the pop loop.
                let echo = crate::tty::TTY_TERMIOS.lock().c_lflag & 0x8 != 0; // ECHO
                let mut writer = if echo {
                    Some(crate::drivers::graphics::console::WRITER.lock())
                } else {
                    None
                };
                while buf.len() < n {
                    if let Some(c) = crate::tty::TTY_INPUT.pop() {
                        buf.push(c);
                        if let Some(w) = writer.as_mut() {
                            // Echo typed input back to the console, mirroring
                            // the write path's byte emission (no [TTY0W] diag —
                            // that is for explicit writes only). Lock held once
                            // for the whole drain, not per byte.
                            w.write_byte(c);
                            crate::serial_putc(c);
                        }
                    } else {
                        break;
                    }
                }
                Ok(buf)
            }
```

Notes:
- The `let echo = ...` binding computes the flag *and drops the
  `TTY_TERMIOS` guard* at the end of that statement, so the lock is never
  held across the `TTY_INPUT.pop()` loop or the WRITER/serial emit — no
  lock-ordering hazard with `TTY_INPUT` or `WRITER`.
- **Enter echo is `\r` only.** Input-time `\n`→`\r` conversion (tty.rs)
  means the echoed Enter returns the cursor to column 0 without a line
  advance on the framebuffer console. Login treats `\r` as submit and
  writes its own `\n` for the next prompt, so the getty flow works — but
  any *other* reader that treats `\r` as literal text would see its line
  overwritten. Acceptable for the single getty; noted for future readers.
- The ECHO mask `0x8` must match login's `ECHO` const (`login/src/main.rs`,
  `const ECHO: u32 = 0x8;`).
- `c` is `u8`; `write_byte`/`serial_putc` take `u8` — no cast needed.
- `\r`/`\n` handling already happens at input time (`tty.rs` converts `\n`
  → `\r` before pushing), so echo emits what the reader sees.

## 4. Verification plan (once applied)

1. **Build:** kernel release with the change; workspace clippy `-D warnings`.
2. **Userspace (no change needed):** login already does
   `TCGETS` → clear ECHO → `TCSETS` around `read_password`. Re-verify
   `tests/test_login_flow.py` still passes (it pins the shadow parsing +
   execve path, not the kernel ioctl).
3. **Runtime (QEMU):** boot the ISO; at the getty, type `root`, then a
   wrong password. With ECHO honored: the username echoes (ECHO set), the
   password does **not** echo (login cleared ECHO before `read_line`), and
   the serial log contains `root` but not the mistyped password. (Until
   this lands, the guest cannot echo at all — the earlier "echo leak" was
   the *harness's* `send` echoing, masked by `log_user 0`.) **This step is
   contingent on a bootable kernel** — the current kernel does not reach
   userspace (the very reason this is a spec), so it unblocks only once
   the kernel's major change lands.
4. **Round-trip check:** a tiny probe (or manual `TCSETS` then `TCGETS`)
   must return the stored struct byte-identical — proving the state is
   actually persisted, not just acknowledged.
5. **Host gate:** extend the `qemu_shell_test.exp` audit entry for the
   login password send — the `log_user 0` harness masking stays as
   belt-and-suspenders, but the guest no longer leaks via echo.

**Attempt-cap interaction (Aug 12, 2026):** the cap's pause marker is
unaffected by this spec. `Too many failed attempts - pausing 30s` is a
direct serial `print_str` (`login/src/main.rs:33`) — output, not echoed
input — so honoring `ECHO` (or keeping it off) cannot suppress it. The
10-failure flow with the spec applied: each wrong password is suppressed
by `echo_off` while the username still echoes, then the pause message, a
30 s `BACKOFF_NS` sleep, and a fresh `login:` re-prompt. The console cap
probe (`qemu_ade_selftest.exp`) asserts the marker exactly once; the GUI
harness (`qemu_gui_login.exp` audit #17) drives the same 10 wrong
passwords into login-manager's window, asserts the marker fires exactly
once (on the 10th) with no login-manager respawn, and proves the counter
reset with an 11th wrong password (see `session-lifecycle.md` "Post-cap
serial trace").

## 5. Assumptions / open questions

- **Single tty:** one global termios assumes exactly one console tty, which
  holds today (`TTY_INPUT` is already one global queue). When the kernel
  grows per-tty instances, `TTY_TERMIOS` should move into the same
  per-tty struct as `TTY_INPUT`.
- **Who reads `TTY_INPUT` after login? (resolved in favor of the design)**
  `TTY_INPUT` is popped **only** by the devfs Tty0 read arm
  (`kernel/src/vfs/devfs.rs:59`) — i.e. only a program that `read()`s the
  console tty. The post-login session (ade) does **not**: it consumes keys
  via `SYS_GUI_GET_KEY` (`libsarga/src/gui.rs:465`) from the GUI keyboard
  path (`kernel/src/gui/mod.rs:580 handle_keyboard` →
  `gui/window.rs:190`), a separate queue from `TTY_INPUT`. So the only
  `TTY_INPUT` reader is the console getty/login, and login's
  `echo_off`/`echo_on` fully controls what echoes there. The `0xB`
  default is therefore safe; no session-side ECHO clearing is needed.
- **Canonical mode:** this spec only honors `ECHO`. ICANON's line editing
  (backspace erase, `\r` on Enter) remains out of scope — the current
  read path is raw-byte with input-time `\r` conversion.
- **`user_access::copy_from_user` signature** may differ in the kernel's
  in-flight rewrite; adapt to the tree's established pattern at apply time.
- **c_lflag default 0xB vs 0x5:** the change sets ECHO by default so
  interactive typing echoes (matching the login-visible behavior a real
  getty expects) and login's `echo_off` is what suppresses it for the
  password. If the maintainers prefer a non-echoing default console, keep
  `0x5` — login's `echo_off`/`echo_on` still round-trip correctly, and the
  username read no longer depends on the default either (next bullet).
- **Username echo is now default-independent (userspace, Aug 12, 2026):**
  login's interactive username read runs **before** `echo_off` (only the
  password is hidden), so it previously relied on the kernel default having
  ECHO set. `login/src/main.rs` now calls `ensure_echo(fd)` — `TCGETS`,
  `c_lflag |= ECHO`, `TCSETS`, best-effort — before the `login: ` prompt's
  `read_line(0)`. With a default of `0xB` this is an idempotent no-op; if
  the rewrite keeps `0x5`, the username still echoes. The §3.3 mask note
  ("must match login's `ECHO` const") extends to this second call site.
