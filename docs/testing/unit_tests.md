# Unit Testing Approach

SkyOS has no unit-test framework on the bare-metal target (kernel and userspace are `#![no_std]`), but **`libsarga`'s and `ade`'s pure-logic `#[cfg(test)]` modules run on the host**: `cargo test -p libsarga` runs 62 tests and `cargo test -p ade --lib` runs 36 tests with the std test harness. Both crates gate their `no_std` attribute with `cfg_attr(not(test), ..)`; the lang items (panic handler, global allocator, alloc error handler) are gated on the sarga targets (`os = "none"`) so they are absent on the host both under `cfg(test)` and when the crate is a dependency of a host binary — `ade` links `libsarga`, and dependencies are never compiled with `cfg(test)`. The closest equivalents for everything else are the host-side suites and the kernel `self_test` feature.

## Host-Side Suites (`tests/skyos-test-core`)

Algorithms are tested host-side (std available) against reimplementations or mocks:

```rust
// tests/skyos-test-core/src/suites/kernel_alloc.rs
Test {
    name: "buddy_alloc_and_free",
    category: "kernel::alloc",
    run: Box::new(|| {
        let mut alloc = BuddyAllocator::new(1024, 11);
        let b1 = alloc.allocate(0).unwrap();
        let b2 = alloc.allocate(0).unwrap();
        alloc.free(b1, 0);
        assert_result!(alloc.is_free(b1, 0), "page should be free after free");
        // ...
        Ok(())
    }),
}
```

Suites use the `assert_result!` / `assert_eq_result!` macros from `skyos-test-core`. Run with:

```bash
cargo run --manifest-path tests/skyos-test/Cargo.toml -- run
cargo run --manifest-path tests/skyos-test/Cargo.toml -- run --category kernel::alloc
```

Each test runs on its own thread under a per-test timeout (default 30 s via
`--timeout-ms`, 0 disables it): a hung test is marked FAILED with a "timed
out" message instead of stalling the run -- and with it, CI. Panics inside a
test are also caught and reported as failures rather than aborting the runner.

The runner and CLI are themselves pinned by hermetic tests wired into CI:
`cargo test --manifest-path tests/skyos-test-core/Cargo.toml` (timeout,
panic, pass/fail semantics) and `cargo test --manifest-path
tests/skyos-test/Cargo.toml` (`tests/cli.rs` runs the real binary and
asserts unknown flags/subcommands/stray positionals exit non-zero -- a
misconfigured invocation can never be silently ignored).

## Kernel Self-Test Feature

In-kernel unit checks run at boot behind the `self_test` feature (kernel Cargo.toml):

```bash
cargo build --release --features self_test ...
```

`kernel/kernel/src/selftest.rs` registers named tests (`selftest::register("vfs::page_cache_basic", test_page_cache)`). `run_all()` emits TAP to serial:

- `TAP version 13`
- `ok <name>` per passing test
- `not ok <name>` per failure (CI fails)
- `# tests/ # pass/ # fail` summary

CI's `integration-qemu` job boots the ISO with `self_test` enabled and greps the log for `not ok`.

## Test Organization

- Host suites: `tests/skyos-test-core/src/suites/*.rs` (`kernel_alloc`, `kernel_mouse`, `kernel_vfs`, `kernel_futex`, `kernel_paging`)
- On-OS scenarios: `tests/thread_test/src/*.rs` (futex, dac, perm, pipe_signal, sigalrm, sigchld, sigint)
- No per-module `src/**/tests.rs` files exist.
