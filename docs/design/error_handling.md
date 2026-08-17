# Kernel Error Handling Strategy

SkyOS uses `Result` and errno-based error propagation.

## Error Types

There is no single umbrella `Error` enum. Subsystems use their own types:
- **Syscalls**: return `u64` errno directly (see `syscalls/errno.rs`) — `0` on success, a positive errno on failure (e.g. `errno::Errno::ENOENT`).
- **VFS**: `Result<T, ()>` — errors carry no payload; the syscall layer maps `Err(())` to the appropriate errno at the boundary.
- **Block devices**: `Result<(), BlockDeviceError>` (`ReadError`/`WriteError`/`DeviceError`).
- **Drivers**: `Result<(), ()>` or `Result<(), &'static str>` at init.

## Error Propagation

`?` propagates errors within a subsystem; `map_err` converts to the target type at module boundaries.

## Errno Mapping

Syscall handlers return POSIX errno values from `syscalls/errno.rs`. Userspace wrappers in `libsarga` return `Result<T, i64>` (negative errno) and set the thread-local errno via `errno::set_errno` for C-style paths.

## Panic Policy

- `panic = "abort"` in both dev and release profiles; `#![deny(warnings)]`.
- Kernel code avoids `unwrap()`/`expect()` in favor of `Result` (convention in AGENTS.md); boot-time initialization may use them where failure is unrecoverable.
- A `#[panic_handler]` (in `libsarga` for userspace) prints `SARGA OS PANIC` and exits the process; the kernel has its own panic path (double-fault handling via IST in `arch/`).

## Debug Assertions

Debug builds include assertions for invariant violations; these are stripped in release builds.
