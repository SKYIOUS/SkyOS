# Memory Allocator Tests

Memory allocator logic is tested two ways: host-side algorithm suites and in-kernel self-tests.

## Host-Side Buddy Allocator Suite

`tests/skyos-test-core/src/suites/kernel_alloc.rs` reimplements the kernel buddy algorithm and validates:

- Single-page allocation
- Allocate/free with buddy coalescing
- Large block allocation (128 pages)
- Exhaustion (allocation fails when out of memory)
- Fragmentation (allocation succeeds after scattered frees)
- Merge chains (freeing all pages coalesces back to one block)

Run:

```bash
cargo run --manifest-path tests/skyos-test/Cargo.toml -- run --category kernel::alloc
```

## Kernel Self-Test

The kernel `self_test` feature registers allocator invariants that run at boot (e.g. `vfs::page_cache_basic` in `kernel/kernel/src/tests/new_features.rs`). Output is TAP; any `not ok` fails CI. See `docs/testing/unit_tests.md`.

## Slab / Virtual Memory Tests

No dedicated slab or VM test suite exists beyond the boot-time self-tests. There is no `cargo test --lib memory`.
