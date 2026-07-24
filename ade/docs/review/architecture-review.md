# Architecture Review

Reviewer: ADE Team
Date: July 2026

## Module Dependency Check

- Desktop ← depends on: all subsystems (central coordinator)
- WindowManager ← depends on: window types only
- ServiceManager ← depends on: individual services
- IpcServer ← depends on: channel, message types
- Compositor ← depends on: layer enum, Canvas
- Each `desktop_api` module ← depends on: Desktop, IPC permission types
- No circular dependencies found
- All subsystems depend on `Desktop` or are leaf modules

## Ownership

- Desktop owns all subsystems (single-owner pattern)
- IPC server owned by Desktop
- WindowManager owned by Desktop
- Services owned by ServiceManager, owned by Desktop
- Compositor owned by main event loop (not Desktop)

## Allocation Paths

- Most allocations happen in `Desktop::new()` (one-time)
- Per-frame: Vec reuse (clear retain capacity in message drain, tick handlers)
- IPC dispatch: `core::mem::take` pattern (zero-copy message drain)
- Window creation: allocates `AppWindow` on Vec push
- Compositor layer buffers allocated once at init

## Event Flow

- Input → `Desktop.handle_event()` → match on Event variant
- Desktop dispatches to subsystems (wm, start_menu, desktop_icons, etc.)
- No event queue — direct dispatch in event loop
- Events forwarded to subsystems via direct method calls, not secondary dispatch

## Key Observations

1. **Monolithic coordinator**: All 30+ subsystems owned by Desktop. Works for alpha, but grows struct definition.
2. **Dual permission systems**: `perms.rs` (PermissionManager) and `ipc/permission.rs` (PermissionSet) have overlapping concepts. One uses u32 bitmask, the other uses the same pattern. Consolidation opportunity.
3. **Dual clipboard**: `service/clipboard.rs` (active) and `clipboard_service.rs` (scaffold) both exist. The scaffold file should be removed in a future phase.
4. **Dual notification**: `service/notification.rs` (active) and `notification.rs` (scaffold) both exist. Same issue.
5. **Dual power**: `service/power.rs` (active) and `power.rs` (scaffold). Same issue.
6. **Dual session**: `service/session.rs` (active) and `session.rs` (scaffold) + `session_service.rs` (scaffold) + `login_session.rs` (scaffold). Consolidation needed pre-beta.

## Recommendations

1. Remove scaffold files that duplicate active services before beta.
2. Consider splitting Desktop struct into focused sub-structs when it exceeds ~50 fields.
3. Unify permission system (`perms.rs` vs `ipc/permission.rs`).
