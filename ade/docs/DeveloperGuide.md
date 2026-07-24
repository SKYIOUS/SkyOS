# Developer Guide

## Build Instructions

```bash
cargo check          # Verify compilation (no test harness)
cargo build          # Build debug binary
cargo build --release # Build release binary
```

No test harness available — ADE uses `#![no_std]` + `#![no_main]`. Verification is manual or boot-time.

## Running

ADE runs as a user-space application within SARGA OS. It is flashed as part of the boot image. Launch from terminal:
```bash
ade
```

## Coding Conventions

### Environment
- `#![no_std]` — standard library not available
- `extern crate alloc;` — alloc crate for Vec, String
- No `dyn` trait objects — static dispatch only
- No `unwrap()` / `expect()` in production code — return `Result` or handle errors inline
- `pub(crate)` visibility by default — expose only what's needed

### Code Style
- Forward-declare functions rather than grouping by access level
- Reuse `Vec` allocations (clear + retain capacity) in per-frame paths
- `// SAFETY:` comment required on every `unsafe` block
- One-line doc comments on public items

### Naming
- Types: `PascalCase`
- Functions/variables: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`

## Adding New Features

1. Add module declaration in `main.rs`
2. Create struct in new file (e.g., `src/my_feature.rs`)
3. Add field to `Desktop` struct in `desktop.rs`
4. Initialize in `Desktop::new()`
5. Wire event handling in `Desktop::handle_event()` or `Desktop::tick()`
6. Add rendering in `render::render()` or `render/mod.rs`
7. Add snapshot data in `snapshot.rs` if renderer needs it

## Debugging Tips

- Press F12 to toggle debug overlay (profiler overlay)
- Logging: `io::print_str()` from libsarga (kernel syscall)
- Crash diagnostics: `CrashManager` captures panic info
- Profiler: embedded profiler tracks frame timing
- Bring up task manager with Ctrl+Shift+Esc (shortcut manager)

## Key Files

| File | Purpose |
|------|---------|
| `main.rs` | Entrypoint, event loop, render dispatch |
| `desktop.rs` | Coordinator — owns all subsystems |
| `window_manager.rs` | Window ordering, focus, drag |
| `window.rs` | AppWindow struct + drawing |
| `render/compositor.rs` | Layer compositor + Canvas |
| `service/service_manager.rs` | Service lifecycle |
| `ipc/server.rs` | IPC message handling |
| `event.rs` | Event enum |
| `constants.rs` | Shared constants |
| `desktop_api/` | Public API for external apps |
