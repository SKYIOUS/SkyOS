# Userspace ↔ Kernel Facility Ownership Audit

**Date:** Aug 8, 2026 · **Method:** read-only source inspection (no code changed)
**Scope:** userspace crates (`ade`, `libsarga`, `init`, `login-manager`, `vahid`, `sash`)
vs. the kernel's syscall surface (`kernel/kernel/src/syscalls/`, `pty.rs`, `gui/`, `drivers/`).
Every claim carries a file:line citation. Line numbers are against the tree at audit time;
the kernel is mid-rewrite, so re-verify before acting on any kernel-side item.

Purpose: find every place a userspace crate assumes a *userspace service* where the
kernel actually owns the facility (or neither side does), so the docs update has a
single source of truth. This is the systemic follow-up to the vahid `0x7d` incident —
where `SYS_CLIPBOARD` (125) was invoked as if it were mknod — and to the
`kernel-gui-window-fix.md` / `kernel-keyboard-gate.md` evidence docs.

---

## 1. Facility ownership map (verified)

### Kernel-owned (userspace must go through syscalls, never re-implement)

| Facility | Syscall(s) | Evidence |
|---|---|---|
| GUI / compositor (windows, buffers, key/mouse, title, destroy, resize, move) | `SYS_GUI_*` 100–105, 120–124 | `syscalls/numbers.rs:46-50,109-113`; handlers `syscalls/mod.rs:4656,4696,4792,4800,4818,4835,4843,4853` |
| Clipboard storage | `SYS_CLIPBOARD` 125 | `syscalls/numbers.rs:114`; `sys_clipboard` `syscalls/mod.rs:4866` (stores in `COMPOSITOR.clipboard`) |
| Notifications (kernel-compositor rendered) | `SYS_NOTIFY` 126 | `syscalls/numbers.rs:115`; `syscalls/mod.rs:4899`; decay/draw `gui/mod.rs:680-682,803` |
| Audio (beep only) | `SYS_BEEP` 104 | `syscalls/numbers.rs:52-53` (only entry in "Audio Syscalls"); `syscalls/mod.rs:1996` → `drivers::audio::pcspeaker::beep` |
| PTY pair | `SYS_OPENPTY` 210 | `syscalls/numbers.rs:101`; `syscalls/mod.rs:6176` → `pty.rs:45 alloc_pty`, line discipline `pty.rs:100` |
| TTY console `/dev/tty0` | devfs | `vfs/devfs.rs`; `init/src/main.rs:69` mounts `devfs` on `/dev` |
| VFS / files (open/read/write/stat/…, `*at` variants) | 0–5, 217, 257, … | `syscalls/mod.rs:300-700` |
| Network stack | socket 41–52 | `syscalls/mod.rs:3450-4325` |
| GPU / display | `SYS_DRMCTL` 400 + DRM ioctls | `syscalls/numbers.rs:99` (SYS_DRMCTL=400); `sys_drmctl` `syscalls/mod.rs:6026-6140`; `drivers/gpu/virtio_gpu.rs` |
| Password hashing (PBKDF2) | `SYS_HASH` 401 | `syscalls/mod.rs:6141` (`sys_hash`, `HASH_SHA256_PBKDF2`) |
| Power-off / reboot | `SYS_REBOOT` 169 | `syscalls/numbers.rs:98`; `syscalls/mod.rs:5541` |

### Userspace-owned (legitimately, no kernel counterpart)

| Facility | Owner | Evidence |
|---|---|---|
| Session lifecycle / reap / exit protocol | `ade/src/service/session.rs` (LifecycleManager, exit_class) | in-process |
| Window management *policy* (tiling, focus, close/min hover) | `ade/src/core/window_manager.rs` | drives kernel windows via libsarga |
| Launcher / app spawn | `ade/src/core/launcher.rs` | fork + execve |
| Settings / theme / file dialog policies | `ade/src/sec/portal/*` | in-process IPC handlers |

---

## 2. Findings (ranked)

### F1 — Two clipboards; cross-system paste is broken by construction (HIGH)

- **Kernel side:** `SYS_CLIPBOARD=125` (`syscalls/numbers.rs:114`), `sys_clipboard`
  (`syscalls/mod.rs:4866`) keeps one `Vec<u8>` in `COMPOSITOR.clipboard`. libsarga
  wraps it: `clipboard_read/write/len` (`libsarga/src/io.rs:856,868,875`).
- **Userspace side:** `ade` keeps its own `ClipboardManager` (`ade/src/service/clipboard.rs`,
  text + 16-entry history) and serves apps over IPC through the perm-gated portal
  (`ade/src/sec/portal/clipboard.rs` → `ade/src/util/desktop_api/clipboard.rs`,
  `PERM_CLIPBOARD` check).
- **Who touches which:** the kernel clipboard is written **only** by sash's readline
  yank (`sash/src/readline.rs:1277,1300,1381,1394,1520,1539,1549,1557,1564,1582,1594,1607,1623,1636,1645,1654` —
  all `clipboard_write`, **no `clipboard_read` anywhere in userspace**). The ade
  clipboard is read/written only by ade apps via the portal. `ade` itself never calls
  syscall 125.
- **Consequence:** a yank in sash lands in the *kernel* clipboard, which the desktop
  never reads; a copy in an ade app lands in the *userspace* manager, which sash never
  sees. The two clipboards are disconnected state with no bridging path. This is a
  genuine cross-system bug, not just duplication.
- **Security note:** the kernel syscall has no per-app identity; the only authorization
  is ade's userspace `PERM_CLIPBOARD` gate (and sash bypasses even that — it calls
  syscall 125 directly). Whoever owns the store owns the trust model. See also F8:
  the kernel handler itself bypasses `user_access`.

### F2 — Two notification systems; the kernel one is 100% dead (MEDIUM)

- **Kernel side:** `SYS_NOTIFY=126` (`syscalls/numbers.rs:115`), `sys_notify`
  (`syscalls/mod.rs:4899`) pushes `Notification{text, kind, ticks_remaining}` into
  `COMPOSITOR.notifications`; the kernel compositor renders and decays them
  (`gui/mod.rs:49` field, `:368` prune, `:680-682` decay, `:803` draw).
- **Userspace side:** `ade/src/service/notification.rs` (`NotificationManager`: id,
  title, body, urgency, timeout, history, cap 64) rendered by
  `ade/src/render/notification_overlay.rs` (`render/mod.rs:96`), served via
  `sec/portal/notification.rs` → `util/desktop_api/notification.rs`.
- **Who calls what:** **nobody** calls libsarga's `SYS_NOTIFY` wrapper
  (`libsarga/src/io.rs:892` — wrapper exists, zero callers in any userspace crate).
  The ade manager is the only live path.
- **Consequence:** the kernel's notification syscall and its compositor overlay are
  dead in every boot. If anyone ever does call `sys_notify`, two notification stacks
  would draw over each other in the same corner (kernel overlay + ade overlay).

### F3 — libsarga Glass API is unwired against the kernel (MEDIUM, dead code) — **DONE (Aug 13, 2026): `libsarga/src/glass.rs` deleted; `pub mod glass;` removed from `libsarga/src/lib.rs`**

- libsarga reserved syscall numbers **130–134** privately in `glass.rs`:
  `SYS_GLASS_SET_OPACITY=130`, `SET_BLUR=131`, `SET_SHADOW=132`, `FLUSH=133`,
  `POLL=134` (`libsarga/src/glass.rs:12-16`), all call sites discarded results
  (`let _ =` at `:37,42,55`). No consumer existed anywhere in userspace — the
  deletion was a clean `rm` + one `pub mod` line.
- The kernel defined **nothing** at 130–134 (`syscalls/numbers.rs` — the range was
  occupied by 125 CLIPBOARD / 126 NOTIFY / 127 MKFS); the dispatch table
  (`syscalls/mod.rs:684-715`) has no arms for them, so they fell through to the
  default arm, which logs `[SYSCALL] Unknown syscall` and returns `ENOSYS`
  (`syscalls/mod.rs:820-823`).
- **Verdict (pre-deletion):** userspace scaffolding assuming a glass/compositor-effects
  facility that exists on neither side. It compiled, reserved five syscall numbers,
  and could never do anything.
- **Numbers status today (kernel mid-rewrite — `numbers.rs` is a moving target):**
  130, 132, 133, 134 are free; **131 is NOT free — the rewrite's
  `SYS_SIGALTSTACK = 131` (`numbers.rs:192`) landed after this audit was written**,
  so the audit's "nothing at 130–134" is stale. Reservation contract for the rewrite:
  when `numbers.rs` settles, add a `// Reserved (was libsarga glass, F3): 130, 132,
  133, 134` comment so the freed numbers are not silently reallocated — with the
  caveat that **133 is the K9 mknod fallback candidate** (§6 queue), so the rewrite
  should treat the four numbers as separate free slots, not a protected range.

### F4 — DRMCTL argument-shape contract mismatch (HIGH for GPU consumers)

The kernel dispatches `SYS_DRMCTL` with only three forwarded args
(`syscalls/mod.rs:756`: `sys_drmctl(arg1, arg2, arg3 as *mut u8)`), and the handler
signature is `fn sys_drmctl(_fd: u64, request: u64, arg: *mut u8)`
(`syscalls/mod.rs:6026`). libsarga's calls do not match that shape in two places:

- **`set_mode` always fails.** libsarga: `syscall5(SYS_DRMCTL, 0, DRM_SET_MODE, w, h, bpp)`
  (`gpu.rs:88`) → kernel receives `_fd=0, request=0x0105, arg=w`; the SET_MODE arm
  then reads `new_w = _fd as usize` (=0) and `new_h = request as usize` (=0x0105=261)
  (`syscalls/mod.rs:6080-6081`), both out of the `640..=3840` / `480..=2160` ranges
  (`:6082`) → **permanent `EINVAL`**. The kernel's own comment says width/height arrive
  "as direct args from userspace" — an expectation no caller meets.
- **`map_dumb(id)` returns the wrong pointer for any `id`.** libsarga:
  `syscall3(SYS_DRMCTL, id, DRM_MAP_DUMB, 0)` (`gpu.rs:97`); the kernel MAP_DUMB arm
  ignores the id entirely and returns the main framebuffer vaddr
  (`syscalls/mod.rs:6095-6097`). Dumb buffers are `Box::leak`ed
  (`:6058`) with no real mapping table, so "map" is a no-op lie.

Working DRMCTL paths (for contrast): GET_DISPLAY_INFO, CREATE_DUMB, DESTROY_DUMB,
FLIP, PAGE_FLIP, GEM_CREATE, GEM_MMAP, ACCENT_COLOR (0x010A), WALLPAPER (0x010B) —
libsarga passes their struct/color/path in `arg` (`gpu.rs:34,55,70,79,106,115,124,135,153`),
matching the kernel arms. Only the two above are shape-broken.

### F5 — IPC service catalog declares four unserved services (LOW, stale registry)

- `ade/src/ipc/registry.rs:9-17` declares `ServiceId::{Clipboard, Notification,
  Launcher, FileDialog, Settings, Session, Window, Theme, Power}` with a full
  permission table (`:19-39`) and wire mapping to libsarga's `SVC_*` constants
  (`libsarga/src/ipc.rs:12-20`, 9 services).
- The portal dispatcher serves only **five**: Clipboard, Notification, Settings,
  Window, FileDialog (`ade/src/sec/portal/mod.rs:16-20`).
- **Consequence:** `Launcher`, `Session`, `Theme`, `Power` are cataloged and
  permission-mapped but have no handlers — and their `desktop_api` backends were
  already deleted in Phase 1. Dead registry entries; an app granted `PERM_POWER` can
  call a service that answers nothing.

### F6 — libsarga clipboard/notify wrappers drift from desktop usage (LOW)

libsarga ships `io.rs:856-892` wrappers for syscalls 125/126; the desktop uses
neither (F1/F2). The wrappers are only consumers: sash yank (125 write) and nothing
for 126. API surface exists that nothing in the boot exercises on the read side.

### F7 — Correctly kernel-backed paths (KEEP — do not re-implement)

- PTY terminal: `ade/src/core/desktop.rs:299` `pty_fd()` → libsarga `openpty`
  (`io.rs:897`, unpacks `master | slave<<16` matching kernel `(m as u64) | ((s as u64) << 16)`
  at `syscalls/mod.rs:6191`). Kernel-owned; no mismatch.
- Console getty: login reads `/dev/tty0` (kernel devfs) — correct.
- Device nodes: vahid O_CREATs into devfs (`vahid/src/main.rs:79-87`) — correct
  kernel-vfs path (the bogus `0x7d` mknod was removed; see `session-lifecycle.md`).
- GUI windows: `libsarga/gui.rs:423,435,465,474` → `SYS_GUI_*` — kernel-backed.
- PBKDF2 login: `sys_hash` (401) — kernel-backed crypto, correct.
- Audio: only `beep` exists; libsarga `SYS_BEEP` is the sole audio surface — correct.

### F8 — The kernel's 125/126 handlers bypass `user_access`; libsarga swallows errno (HIGH, kernel-hardening prerequisite)

- `sys_clipboard` copies to/from user buffers with **raw pointer derefs**, not the
  kernel's user-access boundary: read `core::ptr::copy_nonoverlapping(comp.clipboard.as_ptr(), buf, …)`
  (`syscalls/mod.rs:4876`) and write `copy_nonoverlapping(buf, new_data.as_mut_ptr(), …)`
  (`:4885`). `sys_notify` builds a slice directly from the user pointer
  `from_raw_parts(text_ptr, len)` after a `while *text_ptr.add(len) != 0` scan
  (`:4906`). Every other user-touching handler uses `user_access::copy_to_user` /
  `read_user_string` — e.g. DRMCTL `GET_DISPLAY_INFO` (`:6048`), WALLPAPER (`:6128`),
  korlang (`:6009`), ioctl (`:2343,2358`). **125/126 are the only syscalls that
  dereference user memory directly**, which makes them inconsistent with the
  kernel's own memory-boundary discipline and unsafe on any build with active
  user isolation.
- libsarga's wrappers swallow errno, but not via a dead check: `syscall1`/`syscall3`
  return `i64` (`libsarga/src/syscall.rs:32,46`), and the kernel's `#[repr(i64)]` errno
  (`syscalls/errno.rs:2`; `EINVAL = -22`) is sign-extended into rax, so the `if r < 0`
  guard in `clipboard_read`/`clipboard_len` (`libsarga/src/io.rs:857-865,875-877`) is
  live and correct — but the wrappers then clamp to `0` (empty) instead of surfacing the
  error via `Error::from_i64`, and `clipboard_write`/`notify` drop the return entirely.
  sys_clipboard today never returns errno (u64 byte counts), so the clamp path is inert
  but defensive; the real fix is routing through the i64-typed errno path
  (`Error::from_i64`, the idiom `flush`/`openpty` already use).
- **Consequence for F1/F2:** any rewire that moves ade clipboard traffic onto
  syscall 125 inherits both problems. The kernel rewrite must harden 125/126 with
  `user_access::copy_to_user`/`copy_from_user`/`read_user_string`, and the
  libsarga wrappers must route through the i64-typed errno path
  (`Error::from_i64`, the idiom `flush`/`openpty` already use).

### F9 — `PtyLineDiscipline.echo` is dead scaffolding (LOW, wire-or-delete)

- `kernel/kernel/src/pty.rs:26-29` defines
  `pub struct PtyLineDiscipline { pub echo: bool, pub canonical: bool }`;
  `Default` (`:31-34`) sets both `true`.
- `pty_read_slave` (`:100-133`) consults **only** `ldisc.canonical` (`:103`).
  The file's only `echo` occurrences are the declaration (`:27`) and the
  default (`:33`) — nothing in the master-write path, an ioctl, or a syscall
  ever reads or sets `ldisc.echo`.
- Echo semantics would belong on the master→slave direction (the slave read
  would surface what the master wrote), but `pty_read_slave` only pops
  `slave.buf` — the field has **no integration point** today.
- **Verdict:** same class as F3 — scaffolding that *looks* like a real echo
  path. A future editor wiring TCSETS `ECHO` (per `kernel-tcsets-echo.md`)
  could mistake `echo: true` for live behavior. The kernel rewrite must either
  wire it into the slave-read path or delete the field; do not leave it as a
  silent `true`.

---

## 3. Recommendations (single-owner decisions)

1. **Clipboard (F1): pick one store — the kernel's. — USERSPACE HALF DONE (Aug 10, 2026).**
   `sys_clipboard` is the only store both worlds can reach (sash on the console and ade
   apps in the GUI). `ade/src/util/desktop_api/clipboard.rs` `copy`/`paste` now call the
   libsarga 125 wrappers **after** the existing `PERM_CLIPBOARD` authorization check
   (userspace gate stays; the kernel store is the single shared buffer — a sash yank now
   pastes into ade apps and vice versa). `ade/src/service/clipboard.rs` keeps its buffer
   purely as the history overlay the panel (`render/overlay.rs` `draw_clipboard`) and
   selftests consume. **Remaining kernel-gated (F8):** harden `sys_clipboard` with
   `user_access` (the raw pointer derefs at `syscalls/mod.rs:4876,4885`), and surface
   libsarga clipboard errno via `Error::from_i64` instead of clamping to 0. The wrapper
   `r < 0` checks are NOT dead (see corrected F8 note) — they are the live i64-typed
   errno detection, kept as defense-in-depth.
2. **Notifications (F2): keep the userspace manager, retire the kernel path.** ade's
   model (id, title/body, urgency, timeout, dismiss, history) is richer than the
   kernel's (text/kind/duration, no id/dismiss). Keep `NotificationManager` +
   `notification_overlay.rs`; mark `SYS_NOTIFY` (126) as deprecated-for-removal in the
   kernel rewrite and never call it. This is the opposite decision from F1 because
   notifications are desktop *policy*, while the clipboard is shared *state*. (The
   kernel path is also the other `user_access`-bypassing handler — F8:4906 — so
   retiring it removes a security inconsistency for free.)
3. **Glass (F3): delete `libsarga/src/glass.rs`. — DONE (Aug 13, 2026).** Nothing
   implemented 130–134; it was the canonical "assumed a facility that doesn't exist"
   case. `pub mod glass;` removed from `libsarga/src/lib.rs`; the freed numbers are
   reserved for the rewrite in the F3 section above (131 is now `SYS_SIGALTSTACK`).
4. **DRMCTL (F4): document the shape contract for the kernel rewrite. — DONE (Aug 10, 2026).**
   Exact function-level fix in `kernel-drmctl-fix.md` (K5): SET_MODE reads a `ModeInfo`
   struct pointer from `arg` via `copy_from_user` (CREATE_DUMB pattern); MAP_DUMB gets a
   real id→vaddr registry (`IrqSafeMutex` + `AtomicU64` ids, `-ENOENT` on miss); libsarga
   `destroy_dumb(id)` carries the id. `libsarga/src/gpu.rs` `set_mode`/`map_dumb` are
   marked UNUSABLE with doc comments pointing at the fix doc.
5. **SVC catalog (F5): delete the four unserved ids. — DONE (Aug 10, 2026).** `ServiceId`
   trimmed to the five served ids (Clipboard, Notification, FileDialog, Settings, Window);
   the permission table, `to_wire`/`from_wire`, and `register_defaults` follow; the orphaned
   `PERM_POWER` const and the portal `_` fallback arm were removed with them. libsarga's
   `SVC_*` ids were renumbered contiguously 0-4 with a comment that re-adding requires a
   portal handler. Every cataloged service now answers.
6. **F6 — partially DONE (Aug 10, 2026).** The 125 wrappers are now exercised by the
   desktop (F1 rewire). The `SYS_NOTIFY` (126) wrapper remains unexercised; per F2's
   decision to keep the userspace manager, mark it deprecated-for-removal in the kernel
   rewrite and never call it from ade.
7. **PTY line discipline (F9): wire or delete `PtyLineDiscipline.echo`.** Only
   `canonical` is live (`pty_read_slave`, `pty.rs:103`); `echo` is never read. If pty
   echo suppression is wanted under the TCSETS-ECHO contract, define its integration
   point in the master-write → slave-buf path; otherwise drop the field in the kernel
   rewrite. Either way, `echo: true` must not be read as evidence of behavior by a
   future editor.

## 4. Open questions

- Does the kernel compositor's notification overlay (F2) render *above* ade's desktop
  window in a way that would visually collide, or is it masked by the full-screen
  desktop window? (`gui/mod.rs:803` draws notifications; `ade` fills its desktop
  window.) — affects whether retiring the kernel path is purely cleanup.
- The clipboard history panel hover exists (`ade/src/core/window.rs:51`,
  `desktop.rs:1483`), but a click-to-paste arm for `ClipboardRow` was not found in the
  click path — panel paste may be unwired or handled elsewhere; verify when F1 lands.
- `libsarga/src/io.rs:918-928` documents Linux-termios ioctl numbers for the console
  (echo suppression for login). Whether `sys_ioctl` (`syscalls/mod.rs:2310`)
  implements TCGETS/TCSETS determines if the earlier credential-echo fix is
  kernel-gated — separate audit item.

## 5. Docs update pointer

This file is the evidence annex for the rebuild plan's architecture section. Add a
cross-reference from `rebuild-plan.md` (Gap 1 note: "GUI reachability is a kernel-side
gate") stating the ownership rule with its discriminator explicit:

**Queue status (Aug 13, 2026):** the clipboard items (F1 rewire completion + F8
hardening) and the dev-node/mknod item are kernel-gated — tracked as **K8 / K9** in
[`session-lifecycle.md`](session-lifecycle.md) §6 with landing conditions (selftest TAP
lines + host pins) pinned there. The rest of this audit remains read-only context.

> **One owner per facility.** Shared cross-process state (clipboard, pty, devfs,
> display, hash) is kernel-owned — userspace proxies or gates it, it does not
> re-implement it. Session policy (notifications, window policy, launcher, settings)
> is userspace-owned — a dead kernel path is either wired or deleted, never left as
> a second implementation.

The rule's discriminator is *shared state vs session policy*: F1 (clipboard) is shared
state → kernel; F2 (notifications) is policy → userspace. F1's rewire and F5's registry
cleanup are safe to schedule in the next userspace phase; F2/F3/F4/F8 are kernel-rewrite
items with userspace follow-ups (delete glass.rs — DONE Aug 13, 2026; delete the SYS_NOTIFY wrapper, fix the
125/126 wrappers' errno handling) that can land immediately without touching the kernel.
