# Syscall Testing

Syscalls are exercised through the on-OS `thread_test` crate (`tests/thread_test/`), which is a `#![no_std]`/`#![no_main]` userspace binary that runs at the login prompt and calls real syscalls through `libsarga::syscall::syscallN` wrappers:

```rust
// tests/thread_test/src/futex_test.rs
fn futex(uaddr: *mut u32, op: u32, val: u32) -> i64 {
    unsafe { libsarga::syscall::syscall3(202, uaddr as u64, op as u64, val as u64) }
}
```

## Test Categories Covered

- **Happy path**: valid-argument syscalls succeed (open/read/write/close, futex, fork/exit)
- **Permission checks**: `dac_test.rs`, `perm_test.rs` exercise EACCES paths
- **Signals**: `sigalrm/sigchld/sigint` verify delivery, `pipe_signal_test.rs` verifies signal-interruptible I/O
- **Concurrency**: `thread_test` uses real threads/fork across scenarios

## Running

Booting to the login prompt and running `thread_test` runs the scenarios. See `docs/testing/integration.md` and `docs/guide/testing.md`.

## Automated Verification

There is no `kernel_test_suite!` macro and no `cargo test --test integration syscall` command. CI's `integration-qemu` job boots the system and checks the log for the kernel `self_test` TAP summary (`not ok` fails the job).
