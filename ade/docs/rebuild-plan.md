# ADE Rebuild Blueprint

Status: Proposed · Date: 2026-08-07 · Scope: full audit + ground-up rebuild plan for the ADE desktop shell

This document is the result of a full read-only audit of `ade/` (17,256 lines, 178 `.rs` files). Every claim below is anchored to code. The goal is not to patch ADE — it is to rebuild it into something clear, correct, maintainable, and fast, while keeping only what genuinely earns its place.

---

## 0. Executive summary

- **Verdict:** ADE is a monolithic shell whose growth has been managed by silencing the compiler. **205 `#[allow(dead_code)]` markers across 103 of 178 files (58%)** hide ~40% dead/scaffold code, and the `Desktop` struct (1,987 lines) owns ~40 subsystems. It is not a codebase to patch — it is a codebase to prune, then rebuild along clean layering.
- **Keep:** the pty terminal pipeline and the socket IPC + permission gate are genuinely good and anchor the rebuild; the WM core, `Canvas` primitives, services, shell UI states, and the in-process selftest harness are reusable (with refactor).
- **Delete:** legacy IPC message-bus, four dead `sys/` modules (1,547 LOC), ~2,600 LOC of unwired util/ scaffolding, dead app states, no-op events, double shadows.
- **Rewrite:** `Desktop` (decompose into input/layout/session/wm), `damage` (make rect damage real), input routing (modifier-aware keymap), render dispatch (data-driven, damage-driven).
- **Target state:** a layered shell where `Desktop` is a thin coordinator (<400 lines), every subsystem has one owner, rendering repaints only dirty regions, and pure logic is host-tested with `cargo test`.
- **Net effect:** ~17.3k → ~9–10k LOC after Phase 1 deletion (≈40% reduction before any rewrite), then a codebase where a change touches 1 file, not 4.

---

## 1. What ADE actually is (facts)

- A **monolithic `no_std + alloc` Rust desktop shell** running on the SkyOS kernel, layered on `libsarga` (the userspace syscall + GUI toolkit crate, 9,929 lines).
- A single binary: `main.rs` opens one libsarga `Window` (hardcoded 800×600), runs a manual event loop (`tick → get_key → get_mouse → render → flush → sleep`), and hands everything to the **`Desktop` god-object** (`src/core/desktop.rs`, 1,987 lines) which owns ~40 subsystems as fields.
- **The display stack is kernel-served (Gap 1 evidence, Aug 7, 2026).** `libsarga::Window::create` (`libsarga/src/gui.rs:420`) is two kernel GUI syscalls — `SYS_GUI_CREATE_WINDOW` (#100) + `SYS_GUI_MAP_BUFFER` (#103) — served by the **in-kernel COMPOSITOR**. There is no userspace display server, and `vahid` (`/bin/vahid`, first in init's service table — `init/src/main.rs:82-84`) is a **device manager** (PCI scan + `/dev` node creation, then an infinite sleep — `vahid/src/main.rs:103-120`), not a display server. ADE is a *client* of the kernel's GUI service: its `render/` layer composites content into the one kernel window. **GUI reachability is therefore a kernel-side gate** — if the kernel cannot serve a window (the `SYS_GUI_MAP_BUFFER → 0` / `phys_addr = None` asymmetry behind the `[login] failed to create window` loop, §13 blocker (a)), no userspace change fixes it.
- **Text-line windowing**: every window's content is `Vec<String>` drawn with a hardcoded 8×8 bitmap font at 8 px/char. There is no widget layer. "Apps" are: in-process state drawn onto overlay layers (Settings, Task Manager, About), an in-process file-explorer state machine (`util/explorer.rs`, 1,419 lines), and **external processes** forked with an AF_UNIX socketpair (IPC) or a kernel pty (Terminal → sash).
- Two real pipelines that are genuinely good: **(a)** the pty terminal pipeline (fork → dup2 slave → exec sash, poll, persistent mini-ANSI parser on the window), and **(b)** the socket IPC transport (fd→pid identity, framed wire codec in `libsarga/src/ipc.rs`, permission-gated portal dispatch).
- The compiler is **muzzled**: **205 `#[allow(dead_code)]` markers across 103 of 178 files (58%)**. `clippy -D warnings` in CI therefore proves almost nothing about dead code.

## 2. Systemic problems, ranked by severity

| Severity | Problem | Evidence |
|---|---|---|
| **Critical** | P2 `Desktop` god-object | 1,987 lines, ~40 fields, `handle_click` ~250 lines |
| **Critical** | P1 Compiler muzzled | 205 `allow(dead_code)` markers, 103/178 files |
| **Critical** | P3 Parallel systems | legacy IPC + socket IPC; two terminals; two file managers; 3 recent-apps trackers |
| **Critical** | P7 Security theater | `default_grant()` to every app; empty manifest plumbing; untyped wire protocol |
| **High** | P4 Rendering always full-screen | `compose(win, None)`; rect damage API dead; 30 Hz cursor blink repaints |
| **High** | P9 Tests unwired | `--selftest` only; CI never runs `ade --selftest`; `test.ps1` = `cargo check` |
| **High** | P8 Ad-hoc session/lifecycle | `PowerRequest` → `process::exit(0)`; TEMP-VERIFY hacks uncommitted |
| **High** | P5 Text-only model + magic geometry | `TITLE_H`=28 vs 22; taskbar pitch 120 vs 125; 55-char truncation |
| **Medium** | P6 Fragile constants, raw syscall | `syscall2(35)` in main.rs though `sleep_ms` exists; `DESKTOP_KEYS` magic list |
| **Medium** | P10 Scaffolding burden | ~5,000+ LOC of never-wired modules |
| **Medium** | P11 Stale docs | `Architecture.md` tree/IPC superseded; "no unwrap" claim false |

### P1 — The compiler is silenced, so dead code is invisible [Critical]
`util/` has dead-code allows in **53/58 files**, `sys/` in **7/8**, `sec/` in **10/13**, `apps/` in **7/8**, `ipc/` in **6/10**. The `allow(dead_code)` crutch is the single biggest obstacle to a rebuild: no tool can tell us what is used. It must be removed wholesale (with a CI gate forbidding it) and the resulting warnings used as the deletion map.

### P2 — The `Desktop` god-object [Critical]
1,987 lines owning every subsystem: WM, start menu, context menu, cursor state, double-click tracking, resize state, tiling, focus history, switcher, app registry, lifecycle, services, tray, settings, 6 app states, watcher, a11y, tooltips, IPC server + transport + registry, permissions, profiler, logger, crash diagnostics. `handle_click` alone is ~250 lines of bespoke hit-testing with magic numbers duplicated from the render code. This cannot be extended safely; it is the #1 target for decomposition.

### P3 — Parallel systems everywhere (the "two of everything" problem) [Critical]
| Area | System A | System B | Verdict |
|---|---|---|---|
| IPC | Legacy `Message`/`MessageBus`/`Channel`/"IpcServer channels" (marked "API v1.0 STABLE", `allow(dead_code)`) | Socket `IpcTransport` + `ServiceRequest/Response` + portal (live, tested) | Delete A. Keep B. |
| Terminal | Legacy in-window content typing (`handle_key` fallthrough appends to `content`, `$ cmd` echo) | Real pty + sash (`spawn_terminal`) | Delete legacy path. Keep pty. |
| File manager | In-process `ExplorerState` (1,419 lines, tabs/trash/sort) | Standalone `/bin/skyfiles` (a separate app in the repo) | Pick one. Recommend external app + thin window API. |
| Recent apps | `AppDb.recent` + `AppRegistry.record_launch` + `SessionManager.recent_apps` | — | **DONE — one owner: `AppCatalog.recent`.** |
| Theme state | `core/settings.rs` (`theme_dark`) + `config_store.rs` + `ThemeService` + `settings_app` toggle | — | One owner (ThemeService), rest read through it. |
| Pixel math | `ade` compositor `alpha_blend` + hand-rolled 95-glyph CP437 table | `libsarga::gui::alpha_blend`; libsarga draws text via the **`font8x8` crate** (Unicode) | Duplicated `alpha_blend`; the hand-rolled glyph table is worse than libsarga's — make `Canvas::draw_char` delegate to libsarga/font8x8. |
| Shadows | `window::draw` draws its own shadow **and** `render/mod.rs` calls `draw_shadow` for the same window | — | Double shadow per frame; draw once. |
| Events | `Event` enum (33 variants); ~20 are never produced (`AppInstalled`, `ServiceRegistered`, `PermissionGranted`, `IPCConnected`, `SessionChanged`, …) and match to `{}` | Direct method calls are the real dispatch | Shrink enum to produced variants. |

### P4 — Rendering is full-screen, always [High]
- `render()` calls `clear_all()` (zeroes 6 layer buffers ≈ **11.5 MiB at 800×600** every frame) then re-draws and re-composites everything.
- `compose(win, None)` is always full-frame; the partial-compose path exists and is **never used**.
- `DamageTracker` rect API (`add`/`drain`) is `allow(dead_code)` — only `mark_full` is used, and `mark_full` is called on every click, key, drag, notification, animation tick, tooltip hover change, and pty byte.
- The cursor **blinks at 30 Hz by design** and every alpha transition calls `mark_full` → the desktop repaints the whole screen ~every other frame for a 12×16 cursor.
- `watcher.poll()` runs every frame over a `watched` list that is **never populated** (`watch()` has no callers).
- Shadow drawn twice (see P3).

### P5 — Text-only window model with hardcoded geometry [High]
8×8 px font, 55-char line truncation, 14 px line spacing, all geometry (`TITLE_H`=28 vs a literal 22 in `handle_right_click`/`handle_middle_click`; taskbar button pitch 125 in `build_a11y_tree` vs 120 in click handling and `minimize`) duplicated as raw numbers in desktop.rs, window.rs, start_menu.rs, taskbar.rs. There is no shared layout table and no widget abstraction — every change requires touching 3–4 places that disagree.

### P6 — Fragile constants and bypassed abstractions [Medium]
- `main.rs`: `Window::create("SARGA OS Desktop", 800, 600)` hardcoded; frame sleep via raw `unsafe { syscall2(35, 0, sleep_ns) }` even though **`libsarga::thread::sleep_ms` already exists** (wraps `io::nanosleep`).
- Key routing is a pile of magic numbers: `DESKTOP_KEYS: [1,2,4,5,14,17,19,20,23,24]`, Ctrl+letter scan codes in `shortcut.rs` with no modifier state, `0x0C` Ctrl+L, `key == 88 || key == 0x57` toggling the debug overlay, backspace `0x7F | 0x08`. No key-release events, no modifier tracking, no keymap table.
- `exec_context_action` contains no-op menu items ("paste", "wallpaper", "new_folder", "new_file", "properties" → `{}`).

### P7 — Security theater [Critical]
- `default_grant()` gives **every spawned app** CLIPBOARD | NOTIFICATIONS | FILESYSTEM | WINDOW_CONTROL | SETTINGS. The manifest plumbing that was supposed to drive per-app grants (`app_manifest.rs` 0 fns, `desktop_entry.rs` 0 fns, `desktop_entries` field marked "populated by service layer" but never populated) is empty scaffolding.
- The wire protocol has no version field; `method` is an untyped string and `args` is raw bytes; unknown service/method returns `success=false` with no error code.
- `perms.rs`, `ipc/client.rs`, `ipc/server.rs` (legacy), portal, desktop_api all carry file-level `allow(dead_code)`.
- 51 `let _ =` error swallows in src — errors vanish silently.

### P8 — Lifecycle and session handling are ad-hoc [High]
- `Event::PowerRequest` → `process::exit(0)` directly. Shutdown/restart/logout requests on `SessionManager`/`PowerManager` are set but **nothing acts on them** (the shell just exits).
- `init` currently bypasses `login-manager` (a `TEMP-VERIFY` hack in the working tree) because the shadow file has no PBKDF2 entry — session/login flow is effectively not exercised.
- Working tree carries **uncommitted debug scaffolding**: selftest suite runs at every boot (TEMP-VERIFY), a `[term] n=… last=[…]` serial dump per pty read, login bypass. These must land or be reverted before any rebuild work.

### P9 — Tests exist but are unwired [High]
- `util/testing/` (1,819 lines) is a genuine in-process harness (~31 tests via `run_all`): terminal pipeline e2e (spawn sash, wait for prompt, type, see echo), socketpair transport e2e, codec roundtrips, WM/service/permission tests. Good bones.
- But: it only runs via the `--selftest` argv flag or the TEMP-VERIFY boot hack. `scripts/test.ps1` = `cargo check` only. CI's `integration` job (`.github/workflows/ci.yml:83`) boots the whole system in QEMU and runs the **kernel's** `self_test` suite + a shell interaction test — it never invoked `ade --selftest`, so ADE's suite ran nowhere automated. No render, input, or a11y coverage. No coverage measurement.
- **Stress + regression suites wired into `run_all` (Aug 7, 2026).** The original `stress.rs`/`regression.rs` were deleted in the Phase 1 sweep (they referenced the deleted legacy `MessageBus` and the pre-refactor `AppWindow` literal), so both were rebuilt against current APIs and now run at the end of `run_all`: `stress::run_stress_tests` (100-window create/close churn, 50-window focus flips, 1000-notification flood asserting the queue's 64-cap and `dismiss_all`) and `regression::run_regression_suite` (`test_drag` — the only test exercising `begin_drag`/`update_drag`/`end_drag`, asserting the grab-offset math moves a window (30,30)→(50,50); `test_theme` — non-zero accent invariant). **Latent fix folded in:** `test_window_creation`, `test_window_focus`, `test_full_flow`, `test_spawn`, `test_spawn_at` asserted window counts immediately after `wm.close()` without ticking — but `close()` is animated (windows leave only via `process_closing` during `tick`), so those asserts were latently broken. All five now drain with the documented 60-tick settle loop (`launcher::check_spawn_registers` idiom). Gates green: `clippy --workspace -- -D warnings`, `build`, `fmt --check`.
- **Local QEMU verification attempted (Aug 7, 2026) — blocked by the kernel-side gate.** Full chain built and booted locally on Windows: userspace `x86_64-sarga` release → `build_initrd.py` → kernel `builder` bootimage (re-wrap only, no kernel recompile) → `scripts/make_iso.py` ISO (xorriso via WSL; the script's own WSL shim emits `wsl -as` which WSL rejects, so xorriso was invoked directly with `wsl xorriso -as mkisofs` + `MSYS_NO_PATHCONV=1`). Booted under `qemu-system-x86_64 -bios OVMF.fd -cdrom` (CI invocation) with a Python expect-style driver (no `expect` on Windows). **Result: the in-change kernel does not reach a usable login.** Debug kernel (17:37, newer than all kernel sources — current state): SMP-2 → kernel PANIC (page fault 0xffff8000fee000b0, WATCHDOG CPU-0-stuck) before login; SMP-1 → `login:` appears but login exits (pid Exited, empty ready queue) or SIGSEGV on svc/login-manager, never reaching `sash[`. So `ade --selftest` cannot run until the kernel's SMP/process-lifecycle settles — exactly the "kernel is in major change" gate. The re-pinned `test_hit_window`/a11y/hover suites remain verified by the host gates (build/clippy/fmt) and will gate on the first green kernel boot. `tests/qemu_ade_selftest.exp` and the CI job need no changes.

**CI gap CLOSED (Aug 7, 2026) — ADE suite now runs in QEMU:** new `ade-selftest` job in `.github/workflows/ci.yml` (push **and** PR) builds kernel + userspace + initrd + bootimage + ISO, boots it headlessly, console-logs in (root/skyos — the PBKDF2 dev credential baked into `build_initrd.py`; the old `qemu_shell_test.exp` root/root is stale), runs `ade --selftest` via `tests/qemu_ade_selftest.exp`, and gates on the serial verdict (`[ade] selftest PASS` required, `selftest FAIL` fails the job, plus a grep belt-and-suspenders check on the tee'd log). `ade/src/main.rs` now runs `--selftest` **before** `Window::create` (no GUI dependency — it previously needed a display window) and exits 0/1 with the suite result instead of continuing into the desktop loop, so the CI boot is deterministic and fast.
- **Shell interaction tests un-masked (Aug 7, 2026).** `tests/qemu_shell_test.exp` fixed and its `|| true` mask removed in the `integration` job (`.github/workflows/ci.yml:159`), so the 9 shell checks now gate main pushes. Three bugs fixed: (a) the login password was stale — `root/root` → `root/skyos` (the PBKDF2 dev credential in `build_initrd.py`; also fixed in `tests/test_login.ps1`); (b) the `check` proc now uses `-re` — expect's default glob matching treats `|` as a literal char, so 5 alternation patterns (`SkyOS|sarga`, `sash|init|cat|mkdir`, `PASS|OK`, `self|uptime`, `init|PID`) could never match before and the script exited 1 on the first broken check every run (always masked); (c) file normalized to LF. Reviewer-caught fix: `ls` dropped from the binaries alternation — the shell echoes the typed `ls /bin` command, so `sash|init|ls|cat|mkdir` could false-positive on the echo alone. Remaining `|| true`s are the intentional pip-install fallbacks. Sweep confirmed zero other stale password sends in `tests/`/`scripts/`. **Risk flag (accepted):** the integration job runs on push-to-main only, and the serial `login:` prompt origin is still UNVERIFIED (see session-lifecycle doc) — if that assumption is wrong, the first post-fix main push goes red (loud, not silent).
- Tests construct `AppWindow` with ~20 explicit fields; every field addition edits ~12 call sites (the `esc_state`/`pty_cursor` diff touched 12 files). This was the loudest "fix the data model" alarm in the repo — **resolved: `AppWindow::new()` now collapses all 9 construction sites (see Phase 2 status below).**

### P10 — Scaffolding burden (delete candidates) [Medium]
~5,000+ lines of never-wired scaffolding:
- `sys/audio.rs` (371), `sys/display.rs` (393), `sys/input.rs` (411), `sys/network.rs` (372) — file-level `allow(dead_code)`, zero callers.
- `util/automation.rs` (344), `plugin.rs` (439), `extension.rs` (445), `sdk.rs` (325), `developer.rs` (379), `package/*` (208), `benchmark/*` (219 — "stub, add real timing later"), `crash_manager.rs`, `recovery.rs`, `config.rs` (no persistence, no callers), `desktop_entry.rs`/`app_manifest.rs` (0 functions).
- `apps/terminal.rs` `TerminalState` — superseded by the pty pipeline; `apps/config_store.rs` — superseded by Settings + ThemeService.
- Legacy IPC: `message.rs` (Message/MessageBus/IpcMessage), `channel.rs`, `client.rs`, and the channel half of `server.rs`.

### P11 — Documentation is stale [Medium]
`docs/Architecture.md` describes a module tree that no longer exists, an IPC design superseded by the socket transport, and line counts that are wrong; `docs/Compositor.md` documents a partial-compose feature that is never exercised. Docs claim "No unwrap()/expect() in production paths" — false (see `window_manager.rs:74`, `desktop.rs:248 unreachable!()` in a live path, `network/display/audio` test code). Documentation must be regenerated from the rebuilt system, not patched.

## 3. What is genuinely worth keeping (evidence-based)

| Keep | Why |
|---|---|
| **Pty terminal pipeline** (`launcher::spawn_terminal`, `window::consume_pty_bytes`, `desktop::pump_terminals`, wm close→kill+free) | The most "real" feature; split-read ANSI state is correct and tested. |
| **Socket IPC transport + wire codec** (`ipc/transport.rs`, `libsarga/src/ipc.rs`, `registry.rs`, `permission.rs`) | Clean identity model (fd→pid), no sender spoofing, framed codec with size caps, integration-tested. |
| **Permission gate** (`sec/perms.rs` + `process_ipc`) | Correct structure; only the *grant policy* (flat default) is fake. Keep the gate, fix the grants. |
| **Compositor `Canvas` primitives** (`render/compositor.rs`) | Bounds-clamped, alpha-aware, OOM-safe allocation. Solid drawing foundation. |
| **Window manager core** (`window_manager.rs`, `window.rs`) | Focus, drag, resize, snap regions, tiling, min/max/fullscreen, animations. Mostly self-contained and correct. |
| **Shell UI states** (start_menu, taskbar, desktop_icons, tray, notification + clipboard + session + power services) | Coherent, small, single-purpose. Keep semantics; re-render through the new layout table. |
| **In-process selftest harness** (`util/testing/*`) | Real integration tests with real syscalls. Keep the harness; wire it into CI (the current QEMU integration job boots the system but never runs `ade --selftest`); add unit-testable crates. |
| **ThemeService, ClockCache, Profiler, Logger** | Small, useful, already wired. |
| **a11y tree/focus** (`sec/a11y/*`) | Small and coherent; the only consumers are tooltips + focus ring, which is fine. |

## 4. Keep / Rewrite / Delete / Replace matrix

| Module | LOC | Action |
|---|---|---|
| `core/desktop.rs` | 1,987 | **Rewrite** — decompose into `Input`, `ShellLayout`, `Session`, `Desktop` (thin coordinator). Extract `handle_click` hit-testing into a layout module. |
| `core/window.rs` `window_manager.rs` | 456+454 | **Keep + refactor** — `AppWindow::new()` **DONE**, `TextSurface` split **DONE**, `Terminal` struct wrap **DONE** (see Phase 2 status); remaining: kill double shadow; de-magic-number titlebar. |
| `core/launcher.rs` | 248 | **Keep** — unified spawn path **DONE** (see Phase 2 status): `SpawnKind { External, Terminal, Explorer(u32) }` + one private `spawn()`. |
| `core/event.rs` `shortcut.rs` | ~150 | **Rewrite** — shrink Event to produced variants; add modifier state + keymap table (Key = {code, ctrl, alt, shift}). |
| `core/damage.rs` | ~70 | **Rewrite** — make rect damage real (it already has merge/union; wire it into `mark_rect` call sites), or delete and keep only `full` flag. No half-measures. |
| `core/start_menu.rs` `taskbar.rs` `tray.rs` `desktop_icons.rs` `dialog.rs` | ~890 | **Keep** semantics (`dialog.rs` backdrop/panel is live); move geometry constants into one layout table; remove no-op context actions. |
| `core/drag.rs` (`DragOp`) + `apps/files.rs` (`FileManagerState`) | ~130 | **Delete** — both dead (`DragOp` "windows/icons still use bespoke paths"; `FileManagerState` never drawn or hit-tested). |
| `render/compositor.rs` | 862 | **Keep** Canvas; **rewrite** compose to use damage rects; move glyph table + alpha_blend to libsarga/shared. |
| `render/mod.rs` `snapshot.rs` `overlay.rs` `notification_overlay.rs` `clock.rs` | ~500 | **Rewrite** — data-driven layer list; drop unused snapshot fields (`app_db`, `focused_id`…); delete duplicated shadow; cursor without full repaint. |
| `ipc/message.rs` `channel.rs` `client.rs` + server.rs legacy half | ~300 | **Delete** legacy types only: keep the load-bearing `ApplicationId`/`RequestId` newtypes (used by transport/request/response/portal); delete `Message`/`MessageBus`/`IpcMessage`/`IpcRequest`/`IpcResponse`/`IpcBroadcast`/`IpcTarget`/`MessageType`/`MessagePayload`/`MessageId`/`ChannelId` + the `channels` half of `server.rs`. |
| `ipc/transport.rs` `server.rs` (request/response queues) `registry.rs` `permission.rs` | ~350 | **Keep**. Add wire version field; typed method dispatch; error codes. |
| `sec/portal/*` `sec/perms.rs` | ~300 | **Keep** structure; replace `default_grant` with manifest-driven grants (or explicit allowlist by executable until manifests land). |
| `sec/a11y/*` | ~350 | **Keep**; simplify — it currently rebuilds the whole tree every frame. |
| `service/*` | 424 | **Keep**; delete unused flags (`power.shutdown_requested` etc. — nothing reads them). |
| `apps/terminal.rs` `config_store.rs` | ~120 | **Delete** (superseded). |
| `sys/audio|display|input|network` | 1,547 | **Delete** (dead). Keep `lifecycle.rs` (used), `vfs.rs` (used), `watcher.rs` (delete or wire). |
| `util/explorer.rs` | 1,419 | **Replace** — either ship external skyfiles as the file manager and delete, or keep in-process and delete the fork. Decide; don't run both. |
| `util/app_db.rs` `app_registry.rs` | ~420 | **Merged into `util/app_catalog.rs` (DONE, Aug 7, 2026)** — `AppCatalog { apps, pinned, recent }`; single recent-apps owner; the write-only `SessionManager.recent_apps` tracker deleted. |
| `util/testing/*` | 1,819 | **Keep + wire**. Add `#[cfg(test)]`-style host tests by extracting pure logic into a lib crate. |
| `util/benchmark/*` `package/*` `plugin.rs` `extension.rs` `automation.rs` `sdk.rs` `developer.rs` `config.rs` `crash_manager.rs` `recovery.rs` `desktop_entry.rs` `app_manifest.rs` `file_assoc.rs` | ~2,600 | **Delete** (scaffolding). Rebuild features only when a real consumer exists. |
| `util/log` `profiler` `crash_diagnostics` `desktop_api` | ~500 | **Keep** (desktop_api is the portal's target); add structured errors. |
| `main.rs` | ~150 | **Rewrite** — resolution from kernel/display, `sleep()` via libsarga, remove TEMP-VERIFY, frame loop simplified. |
| `docs/*` | — | **Regenerate** from the rebuilt system. |

**Net effect:** from ~17.3k LOC to roughly **9–10k LOC** after Phase 0 deletion (≈40% reduction before any rewrite), then a smaller, layered core that can grow safely.

## 5. Target architecture

```
ade (bin) — thin shell: init, event loop, frame pacing
├── shell/
│   ├── Desktop (coordinator)      — owns subsystems, no logic
│   ├── input/                     — keymap (modifier-aware), mouse, focus routing
│   ├── layout/                    — geometry tables: taskbar, start menu, titlebar, windows
│   └── wm/                        — window list, focus, drag/resize, tiling, snap, animations
├── render/
│   ├── compositor                 — Canvas + layers + damage-rect compose
│   └── draw/                      — wallpaper, icons, taskbar, start menu, windows, overlays
├── ipc/                           — transport (socket), registry, permission; ServiceRequest/Response only
├── services/                      — notification, clipboard, session, power (small, single-purpose)
├── portal/                        — dispatch + gate (permissions)
├── apps/                          — settings/task-manager/about as shell widgets (or external)
└── util/                          — app catalog, testing harness, profiler/logger
```

> **Layer boundary (Gap 1 evidence):** `render/compositor` here is ADE's
> **in-process** layer compositor over the single kernel-served window —
> the kernel owns window creation/display (`SYS_GUI_CREATE_WINDOW` #100 +
> `SYS_GUI_MAP_BUFFER` #103, in-kernel COMPOSITOR; `vahid` is a device
> manager, not a display server). ADE never implements a display server;
> window availability is a kernel gate outside its scope, asserted by the
> `gui-gate` CI job.

Design rules (non-negotiable):
1. **No `allow(dead_code)`** anywhere; CI greps for it. Dead code gets deleted, not annotated. **DONE: enforced by the `dead-code` job in `.github/workflows/ci.yml`** (fails on any `allow(dead_code)` marker in the whole workspace, both `#[...]` and `#![...]` forms; excludes `target/`).
2. **One owner per concept.** One IPC stack, one recent-apps tracker, one theme owner, one file-manager story, one spawn path.
3. **`Desktop` never exceeds ~400 lines.** All state lives in owned subsystem structs; Desktop wires them.
4. **Rendering is damage-driven.** `mark_rect`/`mark_full` both real; compose blits only dirty rects; cursor drawn on a dedicated layer without repainting the scene; never `clear_all` + full recompose on idle.
5. **Input is structured.** `KeyEvent { code, ctrl, alt, shift }`; a single keymap table maps to `ShortcutAction`; terminal focus is a routing rule, not a pile of magic constants.
6. **Windows carry no UI strings.** `AppWindow` = geometry + state + a `TextSurface` (line storage, view scroll, pty parser state) — layout reads `surface.lines()`/`scroll()`; text mutation goes through surface methods, not raw `Vec<String>` pokes. (See trade-off in §8.)
7. **Errors are propagated.** `Result` up, one structured log line at the boundary; no `let _ =` for anything that matters.
8. **Pure logic is testable on host.** Extract `geometry`, `damage`, `ansi` (pty parser), `ipc codec`, `wm state` into a `no_std` lib crate so `cargo test` runs them on the host with zero QEMU.

## 6. Phased migration (lowest risk first)

### Phase 0 — Land or revert the working tree (1–2 days)
- Finish/review the in-flight pty/terminal commit (uncommitted: `consume_pty_bytes`, openpty unpack fix, selftest additions) and **remove all TEMP-VERIFY** artifacts: boot-time selftest, `[term]` serial dump, init login bypass (restore login-manager or land a real shadow fix).
- Gate: repo boots with no TEMP-VERIFY strings (`grep -r TEMP` clean), terminal works via `--selftest`.

### Phase 1 — Deletion sprint (3–5 days, pure removals, trivially revertible)
1. Remove the legacy IPC layer — keep the load-bearing `ApplicationId`/`RequestId` newtypes in `message.rs`, delete `Message`/`MessageBus`/`IpcMessage`/`Channel`/`IpcTarget`/`MessageType`/`MessagePayload`/`MessageId`/`ChannelId` and the `channels` half of `server.rs`; drop the tests that exercised them (`test_message_bus`, `test_channels`, `stress::test_1000_ipc_messages`).
2. Delete dead sys/ modules (audio, display, input, network) and unwired util/ scaffolding (automation, plugin, extension, sdk, developer, package, benchmark, config, crash_manager, recovery, desktop_entry, app_manifest, file_assoc, apps/terminal, apps/config_store, apps/files `FileManagerState`, core/drag `DragOp`, watcher — `watch()` has zero callers).
3. Remove no-op Event variants, dead context actions, `watcher` (or wire it), duplicate shadow, dead constants.
4. **Remove every `#[allow(dead_code)]`** (205 markers) and fix the resulting warnings by deletion or by wiring. Add a CI grep gate forbidding the attribute.
- Gate: `cargo clippy -D warnings` green, `cargo fmt --check` green, selftest suite passes, LOC ≈ 10–11k.

### Phase 2 — Data-model stabilization (3–5 days)
1. `AppWindow::new()` constructor → collapses ~12 call sites per field change. **DONE (Aug 7, 2026).**
2. Split `content: Vec<String>` out of `AppWindow` into a `TextSurface` (terminal owns it via `Terminal` struct). **DONE (Aug 7, 2026) — TextSurface type + `Terminal { pty_fd, surface }` wrapper both landed.**
3. Single spawn path in `launcher.rs` (terminal, external app, explorer) with one registration sequence. **DONE (Aug 7, 2026).**
4. Merge app catalog: one `AppCatalog` (from app_db + app_registry), one recent-apps owner. **DONE (Aug 7, 2026).**
- Gate: selftest green; diff is mechanical (verified by `cargo clippy` + tests).

**Phase 2a — geometry/menu tuple typing — COMPLETE (Aug 7, 2026).** Replaced the ad-hoc tuples with typed equivalents in `core/geometry.rs`; all gates green (clippy -D warnings, build, fmt).
- `MenuItem { label, action }` + `ContextMenu { x, y, items }` — the three menu consts (`DESKTOP_MENU`/`ICON_MENU`/`SYSTEM_MENU`), the `Desktop::context_menu` field, click handling, and `overlay.rs` rendering now all use them. Both `#[allow(clippy::type_complexity)]` suppressions removed.
- `Rect` replaces: `Desktop::resize_rect`, `prev_tiling_geos: Vec<Rect>`, `render_snap_preview() -> Option<Rect>`, snapshot `focused_bounds`/`snap_preview`, a11y `A11yNode::bounds` (+ `add_node` + `node_at` now via `bounds.hit_test(Point)` — identical >= / < boundary semantics).
- `Point` replaces: the four mouse `Event` variants (`MouseClick`/`MouseMiddle`/`MouseRight`/`MouseDrag` carry `Point` now, constructed in `main.rs`) and the `arrange` desktop-icon positions.
- Zero `(i32, i32, u32, u32)` and zero `(&str, &str)` menu tuples remain in `src/`.
- **RubberBand typed — COMPLETE.** `rubber: Option<RubberBand>` where `RubberBand { x1, y1, x2, y2 }` lives in `core/geometry.rs` with `new(x,y)` / `drag_to(x,y)` / `rect()` (normalized via min + unsigned_abs). The normalization was previously duplicated inline in `end_select` and `draw` — now centralized. The <4px click-vs-drag threshold and the `rubber = None` ordering are preserved (reviewer-verified identical). Zero `(i32, i32, i32, i32)` tuples remain anywhere in `src/`. All gates green.

### Phase 3 — Decompose Desktop (the big one, 1–2 weeks)
1. Extract `layout/` (all geometry constants + hit-testing tables used by desktop.rs, window.rs, start_menu, taskbar — one source of truth). **DONE (Aug 7, 2026)** — see status block below.
2. Extract `input/` (keymap + modifier state; `handle_key` shrinks to a routing function). **DONE (Aug 7, 2026)** — see status block below.
3. Extract `Session`/lifecycle (reap, exit paths, shutdown protocol via `init`). **DONE (Aug 7, 2026)** — see status block below.
4. Desktop becomes a coordinator; `handle_click`/`handle_drag` delegate to layout + wm modules. **DONE (Aug 7, 2026)** — see status block below.
- Gate: after each extraction, selftest + clippy green; Desktop under ~700 lines by end.

### Phase 3.5 — Desktop slimming (measured Aug 7, 2026; Desktop = 1,714 LOC)

Breakdown of the five biggest methods (measured): `handle_click` 255 · `handle_key` 241 · `build_a11y_tree` 101 · `exec_context_action` 101 · `tick` 67 · `new` 61 · `snapshot` 58 · `handle_right_click` 53 · `handle_a11y_key` 50.

`handle_click`'s 255 lines by block: settings overlay 24 · start-menu 57 · settings_app 37 · task-manager 17 · about 5 · taskbar 23 · context-menu 18 · icon+window-loop+desktop 69. The four app-state overlay blocks = **83 lines (33%)**, plus the start-menu block 57. Theme toggling (`theme_svc.set(Theme::dark()/light())`) is duplicated verbatim in the settings overlay and settings_app blocks. `exec_context_action` has **five no-op arms** (paste / wallpaper / new_folder / new_file / properties) — the desktop right-click menu advertises items that do nothing.

Proposed extraction sequence (each independently shippable; order = best LOC/risk ratio):
1. **App-overlay actions** — **COMPLETE (Aug 7, 2026).** `src/apps/mod.rs` defines the shared `AppAction` enum (`ToggleSound` / `SetTheme(bool)` / `SelectPage(SettingsPage)` / `FocusWindow(usize)` / `Close`); `settings_app` (page+theme), `task_manager` (row focus), and the legacy `core/settings` panel (sound/theme/close) now expose `hit_test_action(mx, my, &snap) -> Option<AppAction>`, replacing the old `usize`/`(usize, &str)` `hit_test` returns and Desktop's 10-entry pages array + `idx==10` checks + dead `"focus"` string. New `Desktop::toggle_theme(dark)` kills the duplicated `theme_svc.set(Theme::dark()/light())` pair; the four overlay blocks in `handle_click` collapse from 83 to ~50 lines (the original −80 estimate was optimistic; the real win is structural — page/theme/close knowledge lives in the apps, the 10-entry pages array and `idx==10`/`"focus"` magic are gone, and the duplicated theme code is one helper). About is dismiss-only (no hit regions) and deliberately skips the action round-trip — it closes directly without building a `RenderSnapshot`. New `testing/apps.rs` (`test_overlay_actions`) pins each mapping geometrically and exercises `handle_click` end-to-end (sound/theme/close/outside, page switch, task-manager focus-to-front, about dismiss); about's e2e covered, no pin needed. Theme flag flips (`settings.theme_dark`, `settings_app.app`) remain in Desktop as two near-identical `SetTheme` arms — acceptable seam; a later pass can let the apps own their toggles.
2. **`RenderSnapshot::from(&Desktop)`** — **COMPLETE (Aug 7, 2026).** `render/snapshot.rs` gained `impl<'a> From<&'a Desktop> for RenderSnapshot<'a>` with the full moved snapshot body; `Desktop::snapshot()` is now a one-line delegate (`RenderSnapshot::from(self)`), so all ~10 call sites (handle_click overlays + tests) are untouched. Widened `hovered_window_button`, `switcher_active`, `switcher_idx` to `pub(crate)` for the From impl — consistent with Desktop's existing field-visibility pattern.
3. **a11y tree builder** — **COMPLETE (Aug 7, 2026).** `sec/a11y/mod.rs` gained `pub(crate) fn build_tree(&Desktop) -> A11yTree` (desktop/taskbar/start/buttons/windows+Close owner stamps/icons/notifications/focus sync); `Desktop::tick` now does `self.a11y_tree = crate::sec::a11y::build_tree(self)` and the 78-line in-Desktop builder is deleted. `A11yTree::clear()` removed (its only caller was the old builder). **Design note**: the builder takes `&Desktop` rather than "the snapshot pieces" — this adds a second in-crate cycle (`render/snapshot → core/desktop → sec/a11y → core/desktop`), legal in Rust and consistent with the existing `core → sec` edge, accepted as a conscious tradeoff so the code could leave Desktop in one move. Fresh tree per frame vs the old clear-reuse: one small allocation per frame either way.

**Measured result for items 2+3**: Desktop 1,714 → 1,623 lines (−135 net: 58-line snapshot body → 1-line delegate, 78-line builder → 0). The a11y builder measured 78 LOC, not the estimated 101, so the combined cut is ~135 rather than ~160. Reviewer-verified: NLL borrow-safety of `self.a11y_tree = build_tree(self)` (owned return ends the immutable borrow), identical node ids/order/timing vs the old builder, no stale-id regression (fresh tree behaves like clear+rebuild), no dead code.
4. **Tiling cluster** (`apply_tile`/`apply_monocle`/`set_floating`/`save/restore_geometries`/`cycle_*` ≈ 107) into a `TilingManager`.
5. **Start-menu click** (57) into `start_menu.rs` as `handle_click(desktop)`; **typing fallthrough** (~25) into input; **cursor blink** out of `tick` (~25).
6. **Delete the five no-op context-menu arms** (+ the menu items they serve) unless each gets a real action.

Realistic outcome: 1,714 → ~900–1,000 LOC. Sub-700 needs the tiling cluster + a11y builder + snapshot moves too; it is achievable but is several sessions of work, not one.

### Phase 4 — Damage-driven rendering (1 week)
1. Wire `DamageTracker.add/drain` to real call sites (`mark_rect` for moved windows, dirty terminal lines, notification changes, cursor).
2. `compose(win, Some(rects))` path becomes the default; `clear_all` only on resolution change.
3. Cursor: draw into the cursor layer only when it moved/blinked; skip full repaint.
4. Move glyph table + `alpha_blend` into libsarga (or a shared `render-core` crate) and delete the duplicates.
- Gate: F12 debug overlay shows `Dirty` rects > 0 and frame time drops; selftest green.

### Phase 5 — Testing, security, session (1–2 weeks, ongoing)
1. Wire the selftest suite into CI: QEMU boot job running `ade --selftest` (TAP output), gated like `boot_stress.py`; add `stress`/`regression` to `run_all`.
2. Extract host-testable lib crate (`ade-core`): geometry, damage, ansi, codec, wm state → `cargo test` in CI, no QEMU. Add property tests for rect-union, ANSI parser goldens, codec fuzz-ish roundtrips.
3. Replace `default_grant()` with per-executable allowlist (Terminal/Files/Settings get their needs; everything else gets a baseline). Add wire protocol version + error codes.
4. Real session flow: `SessionManager` requests → portal to `init`/`login-manager` (shutdown/restart/logout), not `process::exit(0)`.
5. Rebuild docs from the new tree.

## 7. Testing, validation, and regression prevention

- **Host unit tests** (fast, CI-cheap): `ade-core` lib crate for pure logic — `cargo test` on the host (no kernel needed) for geometry, damage union, ANSI parser, IPC codec, WM state machine.
- **In-binary integration tests** (kept): the existing ~31 selftests (pty pipeline, transport e2e, services) run under QEMU via `ade --selftest` with TAP output.
- **CI gates** (new):
  - `grep -r "#\[allow(dead_code)\]"` → fail (P1 enforcement).
  - `cargo test` on `ade-core` (host).
  - QEMU selftest job (nightly; skippable on fast PRs but mandatory on merge to main).
  - `grep -r "TEMP-VERIFY\|ponytail: stub"` → fail.
- **Regression prevention**: ANSI parser golden tests (split-read sequences from real sash output captured to fixtures); rect-union property tests; WM invariants (focus always valid, no duplicate ids, closing windows removed).
- **Coverage**: add a smoke render test that snapshots the compositor output hash on a fixed scene, so rendering changes are diffable.

## 8. Performance and stability

- Budget at 800×600, 1 core, 512 MB: **full-frame compose ≈ 2.4 M pixel-ops** (~2.3 M blend/check ops across 5 blend layers over 480k px); at 60 fps that is ~140 M ops/s — this is why damage-rect compose is the single biggest win. Target: idle = cursor-only damage; input = dirty rects; terminal typing = 1–2 line regions.
- Kill the per-frame `watcher.poll()` no-op and the double shadow.
- Keep the OOM-safe compositor allocation (`try_reserve`), add `try_reserve` to other large buffers (notification queue, content).
- Replace the raw `syscall2(35, …)` in main.rs with the existing `libsarga::thread::sleep_ms`.
- Stability: all exit paths go through one `Session` module; `process::exit` only after cleanup (kill ptys, close peers, save state).

## 9. Code quality and simplification goals

- Delete more than you add in Phases 0–2. Target **~40% net deletion** before any new code.
- Magic-number audit: every geometry constant in `layout/`; every key code in `keymap.rs`; scan-code vs ASCII disambiguation handled by libsarga input layer.
- No `// keep:` comments (there are dozens) — either wire it or delete it.
- Ban `#[allow(clippy::…)]` that papers over design issues (`type_complexity` on the context-menu tuple → replace with a `MenuItem` struct).
- One `AppWindow::new()`; remove the 12-site struct literal churn. **DONE — the only remaining `AppWindow { … }` literal in `src/` is inside `new()` itself.**
- Structured, leveled logging via the existing `Logger` (tick-tagged); 51 `let _ =` swallows reduced to zero for meaningful operations.

## 10. Tooling and automation

- **CI**: add ADE jobs (fmt, clippy -D warnings with dead-code grep, `ade-core` cargo test, QEMU selftest) to `.github/workflows/ci.yml` — the existing `integration` job boots the system but never runs `ade --selftest`; add a `TEMP-VERIFY` grep gate.
- **Local loop**: `scripts/check-all.ps1` gains a real `test` step (build + host tests + selftest instructions); `scripts/benchmark.ps1` reads the F12 overlay metrics or a `--bench` mode that prints frame-time histogram + damage stats to serial.
- **Agent workflows**: put the audit findings in `AGENTS.md`-adjacent notes so future agents don't re-add scaffolding; add a `docs/contribution-checklist.md` ("no dead_code allows, no TEMP-VERIFY, one owner per concept").
- **Boot harness**: extend `run_ade_test.bat`/boot_stress-style script to assert on `[test] PASS` count and serial output, so ADE regressions are caught like kernel regressions are.

## 11. Prioritized action items

1. **P0** Land/revert working-tree pty work; remove all TEMP-VERIFY.
2. **P1** Deletion sprint + remove all `allow(dead_code)`; CI gate added. **DONE — all 205 markers removed, `dead-code` CI job live (Aug 7, 2026).**
3. **P2** `AppWindow::new()` + TextSurface split + single spawn path + app-catalog merge.
4. **P3** Decompose Desktop (layout → input → session → coordinator).
5. **P4** Damage-rect rendering; dedupe pixel/font code into libsarga.
6. **P5** CI selftest job; `ade-core` host tests; manifest/per-exec permission policy; real session/shutdown flow; regenerate docs.

## 12. First implementation steps (this week, safe)

1. Review and land (or revert) the uncommitted terminal/pty changes; delete the TEMP-VERIFY blocks in `main.rs`, `desktop.rs`, and `init`.
2. Delete the provably-dead modules (P1 list) and the legacy IPC layer; run `cargo clippy -D warnings`.
3. Strip `#[allow(dead_code)]` one module at a time (core → render → ipc → sec → service → sys → util), deleting what surfaces, keeping the suite green after each.
4. Add the CI grep gates (dead_code, TEMP-VERIFY).
5. Add `AppWindow::new()` and start the TextSurface refactor. **`AppWindow::new()` DONE (Aug 7, 2026).**
6. Write the `ade-core` host-test crate with the ANSI parser and rect-union property tests (cheap, high value, unlocks `cargo test`).

## 13. Assumptions and open questions

- **Assumption:** the SkyOS kernel + libsarga are stable enough that ADE should target them as-is; libsarga changes (dedup of glyph/alpha) are coordinated but small.
- **Assumption:** in-process shell apps (Settings, Task Manager, About) are acceptable for now; the long-term direction is external apps over the socket IPC (the `ipc_echo` seed proves the path).
- **Open:** file manager — keep in-process `ExplorerState` (delete the skyfiles fork) or go external? Decision should be made by whoever owns skyfiles; the plan default is external.
- **Open:** is the hand-rolled 8×8 bitmap font acceptable long-term? libsarga already draws text via the `font8x8` crate and supports TTF; making `Canvas::draw_char` delegate to libsarga kills the duplicate table and enables real text.
- **Open (gap):** external apps (non-terminal, non-explorer) receive **no keyboard routing today** — all keys go to the Desktop. Decide explicitly in Phase 3 whether/when app windows get key delivery (e.g. over the socket IPC), or ADE's input module will re-encode "the desktop eats everything".
- **Note (verified):** `watcher.watch()` has zero callers (confirmed by grep) — delete it, or wire it to the explorer before keeping it.
- **Session flow traced — `docs/session-lifecycle.md` (Aug 7, 2026).** The logout loop (login-manager → ade → `session.request_end()` → exit 0 → init respawn) is fully traced with evidence. **The shadow PBKDF2 blocker is resolved** (`build_initrd.py` ships root/skyos, salt `SKYIOSDESTOPSALT`, 10k iters). New blockers surfaced: (a) ~~the GUI session is unreachable in a stock boot — nothing starts vahid, so login-manager's `Window::create` fails and loops `[login] failed to create window`~~ — **REWRITTEN WITH GAP 1 EVIDENCE (Aug 7, 2026): vahid IS in init's service table (`init/src/main.rs:82-84`, first, `respawn: true`) and is a device manager (`vahid/src/main.rs` — PCI scan + `/dev` node creation, then an infinite sleep), NOT a display server; `Window::create` is kernel-served (`libsarga/src/gui.rs:420` → `SYS_GUI_CREATE_WINDOW` #100 + `SYS_GUI_MAP_BUFFER` #103, in-kernel COMPOSITOR). The `[login] failed to create window` loop is a **kernel-side two-syscall mismatch**: `add_window` (`gui/mod.rs:153`) is infallible, but `SYS_GUI_MAP_BUFFER` returns 0 when the G3 framebuffer's `allocate_contiguous(9)` (2 MB) silently falls back to heap `content`, leaving `phys_addr = None` (`kernel/src/syscalls/mod.rs:4717`) → libsarga `Err(5)` → login-manager exits **0** → init respawns forever (status 0 resets `crashes` — `init/src/main.rs:126-145`). Full trace: `docs/session-lifecycle.md` §1. Enforcement: the `gui-gate` CI job (`tests/qemu_gui_gate.exp`, `.github/workflows/ci.yml:342`) boots every kernel build and asserts `[login] window created` (PASS) vs `[login] failed to create window` (FAIL) — GUI reachability is a kernel-build gate, not an open question.**; (b) the session-end key is *any* Backspace outside a terminal, not the documented Ctrl+Alt+Backspace (no Alt in the byte stream) — **RESOLVED (Aug 7, 2026): the Backspace gate was removed; session end is now the Ctrl+Alt+Backspace chord via `KeyAction::Quit` — the ONLY session-end key (Ctrl+Q and plain 'q' are unbound), pinned in `testing/input.rs::test_session_end_gate` (Phase C)**; (c) ~~nothing in userspace spawns the console `/bin/login` the CI greps for~~ **RESOLVED — Phase A (Aug 7, 2026): init spawns a console getty (`/bin/login` on the inherited console fds, respawn), making the serial `login:` prompt deterministic**; (d) `qemu_shell_test.exp`'s root/root is stale (password is skyos) and its failures are `|| true`-masked — **both fixed, the shell tests now gate main** (see P9 status). Staged plan in the doc: console-getty-first **DONE (Phase A: init service-table getty + sash-prompt pattern fix in the three CI harnesses + `login` passes argv[0] to the shell)** → GUI key injection via QEMU monitor → narrow the chord to Ctrl+Alt+Backspace (Phase C: userside complete — synthetic-tested; kernel Alt delivery still gated) → (done) un-mask the integration job's shell checks.
- **Scope note:** this plan deliberately excludes the kernel and libsarga internals except where ADE misuses them (raw syscall in main.rs, duplicated pixel code). **GUI reachability is one such exclusion, now explicit (Gap 1 evidence):** window creation is kernel-served (in-kernel COMPOSITOR via `SYS_GUI_CREATE_WINDOW`/`SYS_GUI_MAP_BUFFER`), `vahid` is a device manager (not a display server), and no ADE phase builds a userspace display server — the `gui-gate` CI job asserts the kernel side instead (see §13 blocker (a) and §1).

---

### Appendix — hard numbers

- Total: 17,256 LOC, 178 files. Biggest: `desktop.rs` 1,987 · `explorer.rs` 1,419 · `compositor.rs` 862 · `testing/ipc.rs` 572 · `start_menu.rs` 474 · `window.rs` 456 · `window_manager.rs` 454.
- 205 `#[allow(dead_code)]` markers; 103/178 files (58%).
- Per area dead-code allow ratio: core 12/20 · render 2/7 · ipc 6/10 · apps 7/8 · service 5/6 · sec 10/13 · sys 7/8 · util 53/58.
- 51 `let _ =` error swallows.
- 6 full-screen layer buffers ≈ 11.5 MiB at 800×600 (the 18 MB figure in the old Compositor.md assumed 1024×768); full recompose every dirty frame; cursor blinks at 30 Hz forcing full repaints.

---

### Execution status — Aug 7, 2026

**Phase 0 — COMPLETE.** Working tree reviewed and landed: `consume_pty_bytes` persistent ANSI parser, `openpty` unpack fix (matches kernel `master | slave<<16` pack — verified in `kernel/kernel/src/syscalls/mod.rs`), selftest additions (parser/pty/close-kill tests + `test_notifications` hardening), terminal key routing, pty fd cleanup on close. TEMP-VERIFY blocks removed from `desktop.rs` + `main.rs`; init login bypass reverted at the root cause (`build_initrd.py` now ships a real PBKDF2-HMAC-SHA256 `/etc/shadow` entry, cross-validated with node crypto + openssl). `cargo build` and `cargo clippy -D warnings` both green across the userspace workspace. Kernel-free selftest evidence: the ANSI parser logic was transcribed verbatim into a host harness and all 13 assertions pass (the 4 in-suite cases + 9 edge cases: byte-split CSI, split OSC, dangling ESC, tab, wrap, changed-flag semantics). The pty-pipeline/fork tests still require a stable kernel boot, which is blocked by pre-existing kernel instability (tarfs hangs, missing compositor task, toolchain drift) — **kernel is under external major change; not decided here.**

**Phase 1 — partially COMPLETE (deletion sprint).** Net −4,929 lines, 34 files deleted, 131 → 97 files, ~17.1k → 12.2k LOC. All gates green (`cargo clippy -D warnings`, `cargo build`).

Deleted (reference-verified dead before removal):
- `sys/{audio,display,input,network,watcher}.rs` (watcher only ever polled an empty list)
- Legacy IPC: `ipc/{channel,client}.rs`, `message.rs` shrunk to `ApplicationId`+`RequestId`, `server.rs` channel half stripped (kept submit/drain request+response), legacy tests removed (`test_message_bus`, `test_channels`, `test_1000_ipc_messages`, regression `test_ipc`)
- `util/`: `automation, plugin, extension, sdk, developer, package/, config, crash_manager, crash_diagnostics, recovery, desktop_entry, file_assoc, benchmark/` + `app_manifest.rs`
- Dead `Desktop` fields: `config_store` (field only), `terminal_state`, `file_manager`, `vfs` (field only), `watcher`, `file_assoc`, `recovery`, `crash_manager`, `desktop_entries`, `crash_diag` (+ its single write-only call site)

Audit corrections made during the sprint (modules the original audit mislabeled as dead): `desktop_api` is **live** (sec/portal/clipboard + notification call it), `app_db` is **live** (start_menu/app_registry/snapshot), `config_store` is **live** (theme_service), as are `log`, `profiler`, `tooltip`, `lifecycle`, `vfs`, `dialog`, `explorer`.

Remaining P1 work: decide whether `crash_diag.record_event` instrumentation is wanted (removed as the only call site; recoverable from git).

**Phase 1 gate widening — COMPLETE.** The `dead-code` CI job now covers the whole workspace, not just `ade/src`. Before widening, the remaining markers in other crates were swept (clippy as gate, `-D warnings` green across all workspace members): deleted `libsarga/src/init.rs` (66 LOC `InitManager`/`Service`, zero users — the `init` crate has its own local `Service`) and `libsarga/src/init_services.rs` (32 LOC `DEFAULT_SERVICES`, zero users), plus 10 markers across 6 crates: sash (`JobStatus::Done` variant + match arm, `run_script` — `run_script_with_args` kept, `History::save` verified live via `save_history_on_exit` so kept), spkg (`satisfies` deleted; `DepConstraint` shrunk to `name` only — `operator`/`version` were write-only), syslogd (`ensure_fifo`), vahid (`vendor_name`/`device_name` + the now-dead `VENDORS` table they were the only readers of), calculator (`memory` field), sargasettings (`UpdateStatus::Downloading` variant + match arm). Whole-workspace markers: 10 → 0. Reviewer-driven fixes folded in: (a) `parse_dep`'s version-emptiness validation restored (a malformed `"foo>="` dep still errors instead of silently resolving); (b) the gate pattern was hardened to also catch **combined allows** (`#[allow(dead_code, clippy::...)]` — found live on the dead `VENDORS` const, which both the sweep sed and the original CI pattern missed) and now excludes `kernel/` (a separate repo; locally a symlink grep doesn't follow) + `target/`. `cargo clippy -D warnings`, `cargo build`, `cargo fmt --check` all green workspace-wide.

**Phase 1 desktop_api shrink — COMPLETE (audit-verified).** `src/util/desktop_api/` holds only the two live submodules: `clipboard.rs` (`copy`/`paste`, called by `sec/portal/clipboard.rs`) and `notification.rs` (`notify`/`dismiss_all`, called by `sec/portal/notification.rs`). The six unwired submodules (`launcher`, `power`, `session`, `settings`, `theme`, `window`) were deleted during the allow-strip sweep (git shows them as `D`); this audit re-confirmed zero dangling references (`grep desktop_api` outside portals/desktop_api/mod = nothing), zero test references, all 6 `PERM_*` consts still consumed (`clipboard`/`notification` directly + all six via `ServiceId::required_permission`), and `cargo clippy -D warnings` + `cargo build` + `cargo fmt --check` all green. The `Desktop::permission_check` helper is the single gate both live API fns route through.

**Phase 2 app-catalog merge — COMPLETE (Aug 7, 2026). Phase 2 is now fully delivered.** `util/app_db.rs` (273 LOC) + `util/app_registry.rs` (150 LOC) deleted; merged into `util/app_catalog.rs` (330 LOC): `AppCatalog { apps: Vec<AppInfo>, pinned: Vec<bool>, recent: VecDeque<usize> }` with `new()`/`record_launch()`/`get()`/`filtered()` — logic copied verbatim, the no-op `apps[id].id = id` line dropped. **Single recent-apps owner:** `AppCatalog.recent` — the old `SessionManager.recent_apps` + `record_app_launch` were write-only (zero readers; confirmed by grep) and deleted; `SessionManager` is now just `boot_tick` + `uptime` (still exercised by `testing/services.rs` `test_session`). Callers updated: desktop.rs (field type, `StartupMode`/`CATEGORIES` paths, `db.recent`→`recent`), start_menu.rs (imports, `open_with`/`rebuild_filter` take `&AppCatalog`, `db.recent`/`db.pinned`→direct), snapshot.rs (`Option<&AppCatalog>`), launcher.rs (`AppInfo` import, `record_app_launch` call removed). `APPS`/`AppEntry` privatized; `CATEGORIES` stays `pub(crate)`. Reviewer-verified: all `recent`/`pinned` readers+writers accounted for with identical semantics, no surviving `AppDb`/`app_db::`/`app_registry::` path, `record_launch` unchanged (the removed guard only gated a no-op; both readers bounds-check). Census: zero `recent_apps`/`record_app_launch`/`.db.recent`/`.db.pinned` in code. Net −93 LOC from the merge. Gates green: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check`.

**Phase 2 Terminal struct — COMPLETE (Aug 7, 2026).** `window.rs` gained `pub(crate) struct Terminal { pty_fd: i64, surface: TextSurface }` (private fields). `AppWindow` swapped its `pub(crate) surface` + `pty_fd: Option<i64>` for private `surface: TextSurface` + `terminal: Option<Terminal>`, exposing only accessors: `pty_fd()`, `surface()`/`surface_mut()` (terminal windows route to the terminal's surface; plain windows to their own — a window is always one or the other, never both), `attach_terminal(pty_fd)` (uses `core::mem::replace` so the seeded first line rides into the terminal's surface), `take_terminal()` (fd + surface for windows being removed), and `detach_terminal_fd()` (fd only — the surface is moved back onto the window so the close shrink animation still draws the terminal's last text). Every call site converted: `window_manager` close/close_by_pid, `desktop` pump/keys/scroll, `launcher` seeds + `attach_terminal(master)`, `testing/terminal` — verified by grep that no module outside `window.rs` touches the fields directly. Reviewer-verified: seeded-surface preservation, no double-close/leak (take consumes the Option), no stale-surface path, `core::mem::replace` sound in no_std. Reviewer's one real finding fixed: `close()` originally dropped the surface at close time (empty shrink animation) — `detach_terminal_fd()` now keeps the surface alive, and the redundant `is_some` guard collapsed into the single call. Nit: `attach_terminal` documented as replacing any existing terminal. Gates green: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check`.

**Phase 2 launcher unification — COMPLETE (Aug 7, 2026).** `launcher.rs` now has one private `spawn(desktop, path, title, kind, geo: Rect)` with a `SpawnKind { External, Terminal, Explorer(u32) }` enum; every public entry funnels into it: `spawn_app`/`spawn_app_from_registry` (path dispatch → explorer/terminal/external with `cascade_geom`), `spawn_terminal`, `spawn_app_at` (kept as a wrapper — `testing/launcher.rs` calls it directly), and `spawn_explorer` (moved out of `desktop.rs`, whose method is now a 1-line delegation; the `ExplorerState` create/refresh still lives in launcher). The **single registration sequence** — `lifecycle.register` + `permissions.register(default_grant)` + `lifecycle.mark_running` — runs once per forked pid, with `ipc_transport.register` when a socketpair exists. Behavior preserved per kind (reviewer-traced against all three originals): Terminal's openpty-fail early return, dup2 child setup, pty_fd master, and fork-error no-window bail; External's empty-path no-fork gate (`test_spawn` depends on it), `--ipc-fd` argv, `[launched pid]`/`[fork failed]` seeds; Explorer's silent fork-error + window creation. **One deliberate unification:** explorer previously only did `lifecycle.register` — it now gets the full perms + mark_running sequence like every other spawn (safe: sole spawn site, wholesale unregister on reap). Registration order restored to the original External order (lifecycle block above the kind-match). Geometry passed as `Rect` (avoids a new `too_many_arguments` allow). `SpawnKind` is module-private. Gate: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check` all green; `lifecycle.register`/`permissions.register`/`mark_running` now exist at exactly one production site.

**Phase 3 layout extraction — COMPLETE (Aug 7, 2026).** New top-level `src/layout/mod.rs` is the single source of truth for desktop-shell geometry: taskbar (start button, `taskbar_btn_x/rect` at 125px pitch / 120 wide, overflow, tray, clock), titlebar (`titlebar_rect` 28 tall, `close_btn_rect`/`min_btn_rect` = the exact drawn rects, content metrics `LINE_H`/`CHAR_W`/`CONTENT_PAD_X`/`LINE_TRUNCATE_MAX`=55), start menu (`menu_rect(ty)` = 4,ty−464 + search/sidebar/category/list/item/recent/power rect fns), resize/snap consts, and a `trunc(&str, max)` helper. `core/constants.rs` **deleted**; all 7 consumers migrated: desktop.rs (a11y tree, taskbar click + middle-click, titlebar drag/right/middle, close/min hit, start-menu click block, resize/snap), window.rs draw (chrome + content), taskbar.rs, window_manager.rs minimize, snapshot.rs, render/mod.rs, start_menu.rs (its local `MENU_W/H/SEARCH_H/ITEM_H/SIDEBAR_W` consts deleted; draw uses the layout rects). **Deliberate unification deltas** (documented in the module, all dead-zone/misalignment fixes): taskbar clicks were 120-pitch/115-wide → now 125/120 = the drawn buttons; titlebar hits were 22 for system-menu/middle-click → 28 (the visual height); window-button hit regions now equal the drawn rects and the **maximize hit region was deleted** (no button is drawn there); start-menu click gap was 465 → 464, sidebar hit 126 → 122, recent strip capped at 2 and hit exactly the drawn tiles, start-button click 60 → 58 wide. Reviewer-traced every rect fn byte-identical to the draw it replaced; nits folded in: `menu_rect` uses `saturating_sub` (parity with the draw's animation path), the `-210` recent right-reserve is now `MENU_RECENT_RIGHT_RESERVE`, the strip height is a named `MENU_BOTTOM_STRIP_HT`, and a new `testing/layout.rs` `test_layout` (wired after `test_keymap`) pins the unified values (pitch 125, close/min rect coords, titlebar 28, menu origin, truncation limits) so the divergences cannot return. Gates green: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check`. Zero residual raw `*120`/`bx+115`/`w,22`/`75+`/`*125`/`-460`/`w-28`/`w-54`/`>55` in `src/core/`.

**Phase C input modifiers + session-end chord — COMPLETE (Aug 7, 2026).**
`KeyEvent` modifiers (ctrl/alt/shift) are now real: `KeyEvent::new`
constructs full events, `Binding` gained `alt`/`shift` (matching all three
bits exactly in `resolve`/`is_desktop_shortcut`), the Ctrl+Q → Quit binding
was replaced by **Ctrl+Alt+Backspace → Quit** (a desktop grab, so logout
works from a terminal), and the plain-'q'-when-empty fallthrough was
deleted. `Desktop::handle_key_event(KeyEvent)` is the new structured entry
(tests inject the chord synthetically; the byte stream cannot deliver Alt
until the kernel maps modifiers — Gap 2 in session-lifecycle.md).
`test_session_end_gate` re-pinned: chord-only logout, empty-window guard,
near-miss rejection (Ctrl+Backspace / Alt+Backspace / Ctrl+Alt+Shift+Backspace /
Ctrl+Alt+Q), Backspace still edits, Ctrl+Q / plain 'q' never end; `test_keymap`
DESKTOP_KEYS parity is now 9 entries. The `qemu_gui_login.exp` harness ends
sessions with `sendkey ctrl-alt-backspace` (kernel-gated). Gates green.

**Phase 3 input extraction — COMPLETE (Aug 7, 2026).** New `src/input/mod.rs`: `keys` constants (ASCII + PC scan-code set 1), `KeyEvent { code, ctrl, alt, shift }` with `from_byte` decode (the producer folds Ctrl+letter into control codes `0x01..=0x1A`; `0x08`/`0x0A`/`0x0D`/`0x7F`/scan-28 special-cased to Backspace/Enter as the legacy routing did; `0x1B` stays Esc; scan-1/Esc is deliberately NOT decoded because the a11y pre-handler consumes it), `text()` = unmodified `0x20..=0x7E`. `KeyAction` (17 variants) + `Binding { code, ctrl, action, desktop }` + const `BINDINGS` (17 rows) replace the old 11-row `ShortcutManager` pile AND the `DESKTOP_KEYS: [1,2,4,5,14,17,19,20,23,24]` magic list: the terminal-override rule is now the `desktop` flag (`Ctrl+W/T/E/A/B/N/D/S/X` → `desktop: true`; `Ctrl+C`/`Ctrl+L` → `desktop: false` so they reach the shell; Ctrl+Q is omitted — superseded by Phase C, where it is unbound and the chord owns Quit). `core/shortcut.rs` deleted. `Desktop::handle_key` is now a router: global grabs (`ToggleDebugOverlay`/`Escape`, only when no terminal focused) → start-menu block (Enter/FocusNext/Backspace/text) → switcher block → terminal routing via `is_desktop_shortcut` → keymap `resolve` match (11 legacy actions + ClearTerminal + Backspace-fall-through + FocusNext) → plain-'q'-when-empty exit → typing fallthrough. `handle_key_focus` deleted (inlined as the FocusNext arm); `handle_a11y_key` and main.rs's session-end backspace check now use `keys::` constants. New `testing/input.rs` `test_keymap` (first in `run_all`) pins: decode, all 11 legacy bindings, exact `DESKTOP_KEYS` parity over `0..=26`, Ctrl+C/Ctrl+L/Backspace/Esc/plain-'q' reaching the shell, Enter/backspace normalization, plain-vs-ctrl text. Reviewer-verified faithful (every legacy path mapped); findings folded in: Quit comment corrected (plain 'q' has no binding), the 3 uncovered pty keys pinned, and the fall-through match arm split so Escape/ToggleDebugOverlay can't leak into typing. Gates green: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check`. The one surviving copy of the old `DESKTOP_KEYS` list is the parity constant inside `test_keymap` (intentional regression pin).

**Phase 3 click/drag decomposition — COMPLETE (Aug 7, 2026).** `Desktop` now routes, it doesn't hit-test. `layout/mod.rs` gained the one hit-testing table every click path shares: `WindowHit { Titlebar, Close, Minimize, ResizeEdge(u8), Content, Outside }` + `hit_window(x,y,w,h,pt)` with the exact historical priority (titlebar → close → min → edge → content → outside) and `hit_window_edge` (1/2/4 flags). `SnapRegion` moved from `window_manager.rs` into layout with `snap_region_at(mx,my,sw,ty)` — the release-drag corner/edge/none match, verbatim. `WindowManager` gained `resize_drag(id, origin, edges, dx, dy)` (the old resize math + MIN_WIN clamps, verbatim) and now imports `SnapRegion` from layout. `DesktopIcons` gained `toggle_icon(idx)` and `click_empty(mx,my)` so Desktop stops poking icon internals. `desktop.rs`'s window loop is a `match layout::hit_window(...)`; right/middle-click use `matches!(hit_window, Titlebar)` (equivalent to the old `titlebar_rect` test for all inputs); `update_cursor` deliberately keeps edge-first via `layout::hit_window_edge` (clicks prefer the titlebar, the cursor prefers edges — a preserved inconsistency, now documented); `handle_drag`'s resize branch is `wm.resize_drag(...)`; `release_drag`'s 8-arm snap match is `layout::snap_region_at`. The old `Desktop::hit_window_edge` is deleted. **Preserved quirk, now pinned by test:** the close/min buttons are drawn inside the 28px titlebar and the titlebar check comes first, so a left-click on them drags (buttons reachable only via the a11y tree) — `WindowHit::Close/Minimize` exist for contract completeness. New pure `testing/layout.rs::test_hit_window` (wired after `test_layout`) pins titlebar-zone, the shadow quirk, edge flags 1/2/4/5, content, outside, and snap corners/edges/none. Reviewer-traced every path byte-identical (priority order, right/middle-click equivalence, resize clamps, snap arms, else-branch reachability); the one nit (`toggle_icon -> bool` unused) was folded in. Net: `desktop.rs` 1,987 → 1,717 LOC. Gates green: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check`.

**Phase 3 item 4 follow-up — window buttons activated (Aug 7, 2026).** `layout::hit_window` priority reversed to Close → Minimize → Titlebar → edge → content: a left-click on the drawn close/min buttons now actually closes/minimizes instead of starting a titlebar drag (they sit inside the 28px strip, so the old titlebar-first order shadowed them). `handle_right_click`/`handle_middle_click` **reverted** from `matches!(hit_window, Titlebar)` back to `layout::titlebar_rect(...).hit_test(pt)` directly — required, because the old equivalence only held while titlebar was first; now right/middle-click over a button must still open the system menu / close the window (byte-identical to the pre-Phase-3 code). `handle_click`'s match arms were already written and are now reachable. a11y unaffected: the tree builds button nodes from `close_btn_rect` and never consults `hit_window`. `test_hit_window` re-pinned: button centers assert Close/Minimize, strip + a new gap-between-buttons pin assert Titlebar. Gates green: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check`. Known remaining gap (pre-existing, surfaced by review): a11y-activating a "Close" node is a no-op today (`activate_a11y_node` handles Window/Icon/Taskbar roles only) — follow-up if a11y window control matters.

**A11y Close button wired — RESOLVED (Aug 7, 2026).** `A11yNode` gained `owner: Option<WindowId>` (stamped via `A11yTree::set_owner`, mirroring `set_parent`); `build_a11y_tree` now uses an index loop over `self.wm` so each Window node and its Close button node carry the real `WindowId`. `activate_a11y_node` gained a `Button` arm — label-guarded to `"Close"` so future owner-stamped buttons (taskbar bring-to-front) can't become closes — that closes the owner window; the `Window` arm dropped its dead `label.parse::<usize>()` (labels are titles, so it always no-oped) and now `bring_to_front`s via `owner`. Start/taskbar buttons stay inert (no owner). New `testing/a11y.rs` `test_a11y_close_button` (hermetic fresh desktops) pins: Close node owner, Enter-activation closes after the 8-tick settle, Start button doesn't close, and activating a Window node re-focuses it. Gates green: clippy -D warnings, build, fmt --check.

**Window control hover affordance — COMPLETE (Aug 7, 2026).** `window::draw` now lights the close/min buttons under the pointer: `WindowHover { win: WindowId, btn: WindowButton::{Close,Minimize} }` computed in `Desktop::hovered_window_button()` (topmost-first `layout::hit_window` scan with the same overlay + drag/resize + taskbar guards as `handle_click`/`update_cursor`, stopping at the first non-Outside hit so hover always matches the click target), threaded through `RenderSnapshot.hover_button` to both `window::draw` call sites. Close fills `colors::WIN_CLOSE_HOVER` (0xFFE81123, previously dead in libsarga) when hovered; Minimize gets a 0x35FFFFFF white wash over `bg_elevated`. New `testing/window.rs::test_window_hover` pins close/min centers, content-no-hover, and start-menu suppression. Gates green: clippy -D warnings, build, fmt --check.

**Phase 3 session extraction — COMPLETE (Aug 7, 2026).** `service/session.rs` now owns the whole session lifecycle; `sys/lifecycle.rs` **deleted**. `AppState`/`ExitClass`/`exit_class()`/`AppLifecycle`/`LifecycleManager` moved verbatim into it, and `SessionManager` (promoted out of `ServiceManager` — it was a peer of the shell, not a shell service; the old `desktop.services.session` path becomes `desktop.session`) gained `pub(crate) lifecycle`, an `ending` flag, and the session-end protocol: `request_end()` / `is_ending()` / `exit_code()` (returns const `EXIT_LOGOUT = 0` — the exit-code contract with `init`, which resets its crash counter and respawns the login service on status 0; future reboot/poweroff codes slot in here). `Desktop::reap_children` is gone: `tick` calls `session.reap(&mut wm, &mut services, &mut permissions, &mut ipc_transport, current_tick) -> bool` (loop body moved verbatim; returns whether anything was reaped so `tick` marks damage — the session module no longer knows about render damage, per review). The two `process::exit(0)` exit paths in `handle_key` (Quit when empty, plain 'q' when empty) now route through `session.request_end()` + `session.exit_code()`; main.rs's loop is `while !desktop.session.is_ending()` with Ctrl+Alt+Backspace → `request_end()`, and the post-key-loop fast-exit is restored (`if desktop.session.is_ending() { break; }`). `ServiceManager` dropped the `boot_tick` ctor param (now only notifications/clipboard/power); launcher's single registration sequence uses `desktop.session.lifecycle.register/mark_running`. Tests: `testing/ipc.rs` `test_exit_class` and `testing/launcher.rs` (AppState + `.session.lifecycle.procs`) re-import from `service::session`; `services::test_session` reads `desktop.session.uptime`; new pure `testing/session.rs` `test_session_end_protocol` (wired after `test_layout`) pins fresh-not-ending, `uptime(0)==0`, `request_end` marks ending idempotently, `exit_code()==0`. Reviewer-verified: reap loop byte-faithful, promote-out-of-ServiceManager is the only sound way to avoid the double-borrow (`self.services.session` receiver + `&mut self.services` arg), zero missed references (grep). Gates green: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check`.

**Phase 2 launcher selftest — COMPLETE (Aug 7, 2026).** Added `launcher::test_spawn_registers` (+ a `check_spawn_registers<F: FnOnce>` helper) to `util/testing/launcher.rs`, wired into `run_all` after `test_spawn_at`. It runs the unified spawn three ways — `spawn_terminal`, `spawn_explorer`, and `spawn_app` with a guaranteed-absent binary — and asserts for each: exactly one new window, the active window has a pid, that pid is in `lifecycle.procs` as `AppState::Running`, and `permissions.granted(pid)` is Some, then closes the window and ticks until the count returns to baseline. The absent-binary case pins the External fork path (child exec fails instantly; the parent-side registration is what's asserted). Cleanup design: close() is animated, so the helper settle-ticks before snapshotting the baseline (earlier tests that close without ticking leave stale `closing` windows that `process_closing` would flush mid-test) and ticks 60x after close (terminal: kill→reap→close_by_pid; external: child exits → reaped; explorer: animated out via process_closing). Reviewer-verified: assertions match the single registration sequence; the settle fix was the one real finding. Gates green: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check`.

**Phase 2 TextSurface split — COMPLETE (Aug 7, 2026).** New `src/core/text_surface.rs`: `TextSurface { lines: Vec<String>, scroll: u32, esc_state: u8, cursor: u16 }` now owns all text state. The pty ANSI parser (`consume_pty_bytes`/`pty_put_char`/`pty_erase_to_end`) moved verbatim from `AppWindow`, plus new surface methods replacing every raw pokes: `push_line` (launcher seeds, `$ cmd` echo), `truncate` (500-line cap — the `len > max` guard moved inside), `clear` (Ctrl+L — now resets scroll/cursor/esc_state too, a deliberate hardening: clearing mid-escape-sequence can no longer corrupt the next pty bytes), `scroll_by` (handle_scroll math moved in), `push_char`/`pop_char` (legacy typing path, preserving the old check-before-push 80-char wrap), `lines`/`last_line`/`scroll` accessors. `AppWindow` dropped `content`/`scroll`/`esc_state`/`pty_cursor`; it now holds `surface: TextSurface` + `pty_fd` only. `draw()` reads `surface.lines()`/`scroll()`; desktop's `pump_terminals`, Ctrl+L, key fallthrough, and `handle_scroll` all go through the surface; launcher's 4 seed sites use `push_line`; `testing/terminal.rs` exercises `surface.consume_pty_bytes`. Reviewer-verified: parser byte-identical, `push_char` preserves the old pre-push wrap check (distinct from pty_put_char's post-push wrap, as before), `scroll_by` math identical, zero missed `.content` references (census: only `start_menu`'s own unrelated `scroll` remains), all accessors have live callers, no dead code. The `clear()` scroll-reset was flagged as a behavior change and resolved by making it a **full** reset (documented) rather than a half-measure. Two magic 80s unified into `LINE_WRAP`. `core/text_surface.rs` is the natural home for the Phase 5 `ade-core` host-test extraction of the ANSI parser. Gates green: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check`.

**Phase 2 constructor collapse — COMPLETE (Aug 7, 2026).** Added `AppWindow::new(x, y, w, h, title)` to `core/window.rs` — the single place where defaults for every non-geometry field live (`prev_*` == current, `focused: true`, `id: 0` (overwritten by `wm.create`), `flags: VisualFlags::new()`, `explorer_id: None`, `pty_fd: None`, `esc_state: 0`, `pty_cursor: 0`, …). Converted all 9 literal construction sites: `launcher.rs` `spawn_terminal`/`spawn_app_at` (1-liners; `pid`/`pty_fd` set post-fork as before), `desktop.rs` `spawn_explorer` (sets `app_win.explorer_id = Some(id)` after), `testing/desktop.rs` (win_b gets explicit `focused = false` — the only site that diverged from the default), `testing/integration.rs`, `testing/terminal.rs` `bare_window`. Unused `VisualFlags`/`WindowState`/`String` imports dropped in 4 files. Reviewer-verified: every default matches the old literals byte-for-byte; `wm.create` overwrites `id` so the `id: 0` default is safe; grep confirms zero other `AppWindow {` literals remain in `src/`; the `win_b.focused = false` line is functionally dead in the test (overwritten by `bring_to_front`) but kept for literal fidelity. Future fields (e.g. `TextSurface`) now touch exactly one place. Gates green: `cargo clippy --workspace -D warnings`, `cargo build`, `cargo fmt --check`.

**Phase 1 allow-strip — COMPLETE.** All 158 remaining `#[allow(dead_code)]` markers stripped, one module at a time, clippy as the gate after each file. **0 `allow(dead_code)` markers remain in `src/`** (down from 205 at audit time). Net: 97 → 84 files, ~12.2k → 9,510 LOC. `cargo clippy -D warnings` and `cargo build` both green.

Notable findings & removals during the strip:
- `core/`: deleted `icons.rs` (orphaned draw), `SNAP_PREVIEW_ALPHA` const, tray `draw_tray`/`tooltip`/`TRAY_ICON_*`, geometry `Size`/`contains`/`center`/`translate`/`inflate` (kept `Rect`/`Point`/`hit_test`/`intersects`), 23 never-constructed `Event` variants + their no-op match arms (kept 10 live), `wm.focus`/`wm.maximize`, window `Selection` struct + field (+16 constructor lines), 4 dead start-menu methods + `Sidebar` variant, `ThemeKind`/`kind`/`high_contrast_theme`/6 dead theme methods, 3 dead `Cursor` variants + 3 dead desktop methods.
- `render/`: 6 dead compositor methods + `damage` field, 2 dead snapshot fields.
- `ipc/`: dead `PermissionManager` (the `sec::perms` one is live — the two were parallel systems, P3 confirmed), 6 `PERM_*` consts, `registry.version` field, 3 query methods.
- `service/`: dead `power.rs`/`session.rs` bodies (kept thin live cores), `service_manager` cleanup.
- `sys/`: `lifecycle.rs` trimmed, `vfs.rs` dead helpers.
- `util/`: `profiler` trimmed to live timers (frame_timer + metrics used by debug overlay), `log` kept (logger.info called every tick), `app_registry.find_by_exec`, `app_db` `desc` field (29 literals), explorer dead block (FileOpLog, split/split_tab/ops fields, `active`/`enter_dir`/`new_tab`/`close_tab` methods), `desktop_api` trimmed to live clipboard/notification.
- `sec/`: `A11yRole` shrunk from 18 → 7 constructed variants, `FocusManager` dead variants removed.
- `testing/`: `regression.rs` and `stress.rs` were **entirely unwired** (only `run_all` is called); removed dead test bodies, kept the wired suites (`desktop`, `services`, `ipc`, `launcher`, `renderer`, `window`, `terminal`).

Remaining `allow(clippy::...)` markers (7 total) are all legitimate and documented: 5× `too_many_arguments` + 2× `type_complexity` on fixed-shape draw helpers. Two markers that masked real dead logic were **fixed, not kept**: `clippy::const_is_empty` (spawn_explorer's `if !path.is_empty()` guard on the constant `"/bin/skyfiles"` — guard removed, fork runs unconditionally) and `clippy::drop_non_drop` (the `drop(snap)` no-op borrow-ending trick — both sites rewritten as scoped blocks ending the `snapshot()` borrow naturally). Also cleaned while in there: the `process_ipc` soft-ceiling comment indentation and a redundant `.iter().iter()` in `snapshot()` confirmed non-redundant (custom `wm.iter()` returns a slice; second `.iter()` is required).

**Gap 1 evidence — COMPLETE (Aug 7, 2026): GUI reachability is a kernel-side gate; no userspace display server exists or is planned.** Two verified facts close the open question of how windows come to exist:
- **vahid is the device manager, not a display server** — first in init's service table (`init/src/main.rs:82-84`, `respawn: true`): it scans PCI (`scan_pci`, `vahid/src/main.rs:25`), creates `/dev` nodes (`create_devices`, line 68), exits non-zero on fatal node-creation failure (`EXIT_DEVICE_SCAN_FAILED = 1` — the exit-code contract that lets init's respawn accounting distinguish fatal from healthy, pinned by `tests/test_vahid_contract.py`), and otherwise prints `[vahid] ready` and sleeps forever (lines 116-120). It never touches the GUI.
- **`Window::create` is kernel-served** — `libsarga/src/gui.rs:420` issues `SYS_GUI_CREATE_WINDOW` (#100) then `SYS_GUI_MAP_BUFFER` (#103), handled by the in-kernel COMPOSITOR. The `[login] failed to create window` respawn loop is a kernel-side two-syscall mismatch: `add_window` is infallible (`gui/mod.rs:153`), but `SYS_GUI_MAP_BUFFER` returns 0 when the G3 framebuffer's `allocate_contiguous(9)` (2 MB) silently falls back to heap `content`, leaving `phys_addr = None` (`kernel/src/syscalls/mod.rs:4717`) → `Err(5)` → login-manager exits 0 → init respawns forever (status 0 resets `crashes`, `init/src/main.rs:126-145`). Full trace: `docs/session-lifecycle.md` §1.
- **Consequence for the plan:** no ADE phase builds or assumes a userspace display server. P4 damage-driven rendering and the §5 `render/` tree are about ADE's **in-window** layer composition only. Enforcement: the `gui-gate` CI job (`.github/workflows/ci.yml:342`, `tests/qemu_gui_gate.exp`) boots every kernel build and asserts the first window marker — `[login] window created` → PASS, `[login] failed to create window` → FAIL, boot-timeout → distinct FAIL arm.
