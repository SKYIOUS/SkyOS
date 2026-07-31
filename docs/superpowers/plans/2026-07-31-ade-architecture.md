# ADE Architecture Integration: Wire IPC, Security, and Lifecycle

Date: 2026-07-31

## Goal

The architecture document (`docs/ade/desktop-environment-architecture.md`, 8 traces)
is an accurate codemap of the ADE's file structure, but traces 3 (IPC dispatch), 4
(permission validation) and 8 (lifecycle) describe subsystems that are **defined but
never wired**. This plan makes those traces actually function end-to-end, fixing
root causes, keeping the crate compiling after every step.

## Verified gaps

1. `sec/portal::dispatch` has zero callers. `desktop.ipc_server` (IpcServer) is
   created but never drained. `ServiceRequest` is never constructed.
2. `desktop.permissions` (`sec::perms::PermissionManager`) is never populated —
   `permission_check()` always returns `false`, so every gated `desktop_api`
   operation silently no-ops.
3. Dual permission systems with **conflicting bit encodings**:
   - `sec/perms.rs`: `PermissionSet { perms: u32 }`, `PERM_SETTINGS = 0x0008`
   - `ipc/permission.rs`: `AppPermission` bitflags, `SETTINGS = 0x0010`
   `desktop.permission_check` passes `AppPermission::bits()` into a
   `sec::perms::PermissionSet`-style check — misaligned even if registered.
4. `ServiceRegistry` created but never populated — `find`/`find_by_permission`
   always miss.
5. Lifecycle: `mark_running`/`mark_crashed` never called; `reap_children` ignores
   wait status (crash vs clean exit); no permission cleanup on reap.

## Approach

Single permission type: `ipc::permission::AppPermission` (already used by every
`desktop_api`/portal caller). `sec::perms::PermissionManager` stores
`AppPermission` sets; the old u32 `PermissionSet`/`PERM_*` constants are removed.

## Tasks

### Task 1 — Unify permission system onto `AppPermission`

- `sec/perms.rs`: `PermissionManager` stores `Vec<(u64, AppPermission)>`;
  `check(pid, perm: AppPermission)`. Delete u32 `PermissionSet` + `PERM_*`.
  `register` returns already-registered state; `unregister` removes.
- `desktop.rs` `permission_check(app, perm)` -> `self.permissions.check(app.0, perm)`
  (drop `.bits()`).
- Update sole other consumer `util/service_manager.rs` (dead scaffold) to use
  `AppPermission::from_bits_truncate`.

### Task 2 — Populate `ServiceRegistry` at startup

- Add `ServiceRegistry::register_defaults()` registering all services present in
  `sec/portal` (Clipboard, Notification, Launcher, FileDialog, Settings, Session,
  Window, Theme, Power) with the required permissions already declared in each
  portal module (reuse the constants, don't re-declare).
- Call from `Desktop::new()`.

### Task 3 — Wire IPC dispatch loop

- `ipc/server.rs`: add `pending_requests: Vec<ServiceRequest>` + `submit_request()`
  + `drain_requests()`. `Message` (channel) path stays untouched.
- `desktop.rs`: `process_ipc(&mut self)` drains `ipc_server`, validates the
  caller against `permissions`, dispatches through `portal::dispatch`, pushes the
  `ServiceResponse` back onto the server's response queue.
- Call `process_ipc()` from `tick()` so requests flow every frame (doc: ~60Hz).

### Task 4 — Permissions + lifecycle at launch/reap

- `launcher`/`desktop` launch path: register a default `AppPermission` set for the
  new pid and call `lifecycle.mark_running()` after fork/exec succeeds.
  `// ponytail: flat default grant, manifest-driven grants when manifests exist`.
- `reap_children`: on wait, distinguish non-zero exit (-> `mark_crashed` +
  crash notification) from clean exit (-> `mark_terminated`); call
  `permissions.unregister(pid)` and `app_lifecycle` cleanup in both cases.

### Task 5 — Regression tests

- Extend `util/testing/mod.rs` with `run_ipc` / add portal-dispatch + permission
  round-trip cases wired into `run_all` (compile-checked; no host runner in this
  codebase).

## Verification

After each task: `cargo +nightly build --target x86_64-sarga.json --release -p ade`
must succeed with no new warnings (`#![deny(warnings)]`). Final pass reviews the
diff and re-runs the build.

## Review follow-ups (post-implementation review)

- **IPC per-frame cap**: `process_ipc` drains at most 64 requests/frame; leftovers
  drain next frame. Soft-real-time ceiling — no frame stalls on a huge queue.
- **Crash classification**: `sys::lifecycle::exit_class(status)` maps the kernel's
  raw wait4 status per Unix convention — `0`=Clean, `1..127`=Error exit, `128+sig`=
  Signal death (kernel `sys_exit(128+sig)`), `<0`=Killed. `reap_children` now marks
  crashed vs terminated accordingly. Covered by `test_exit_class`.
- **Lifecycle leak fix**: `lifecycle.remove(pid)` on reap (procs vector previously
  grew unbounded; nothing read the history).
- **Permission revocation**: `permissions.unregister(pid)` on every reaped child
  (crash, error exit, and the exec-fail path which exits 1 and is reaped next tick).
- **Dead duplicate modules removed**: `util/service_manager.rs` (legacy MessageBus
  ServiceManager, duplicate of `service/service_manager.rs`) and
  `sys/app_lifecycle.rs` (duplicate of `sys/lifecycle.rs`) deleted with their
  module declarations and the unused `Desktop.app_lifecycle` field.
- **Portal panic audit**: all portal handlers + `desktop_api` entries verified
  panic-free on malformed/unknown input — every `match` has a `_` arm returning a
  failed `ServiceResponse`, and all arg parsing uses `unwrap_or` defaults.
- **No new warnings**: ade warnings 80 (baseline) → 79.

## Remaining architectural debt (documented, not fixed)

- Other dead scaffold modules kept: `sys/{notification,power,session,session_service,
  login_session}.rs`, `util/{clipboard_service,automation,benchmark,developer,
  extension,package,plugin,sdk}.rs`, `app_manifest.rs` (never read), legacy
  `MessageBus`/channel IPC in `ipc/{message,channel,client}.rs` (superseded by
  `ServiceRequest` path). Remove when each has no remaining consumers.
- `LifecycleManager.restart` policy (`OnCrash`) is stored but unenforced — no
  auto-restart. Add when a restart decision path exists.
- `status == 128` is ambiguous between `exit(128)` and a signal with `sig 0`;
  inherent to the kernel's 128+sig encoding.
- No cross-process IPC transport reaches `IpcServer` yet — `submit_request` is
  in-process only. The 64/frame cap is the guard for when a transport lands.

## Non-goals

- No new dependencies.
- Not touching wired traces (1, 2, 5, 6, 7).
- Not removing every dead scaffold (out of scope); only those that block a wired trace.
