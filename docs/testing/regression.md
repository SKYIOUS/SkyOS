# Regression Test Suite

There is no dedicated regression test framework or `tests/regression/` directory. Bug-fix verification relies on the existing test infrastructure:

- **Host-side suites** (`tests/skyos-test`) — add a `Test` to `tests/skyos-test-core/src/suites/` when a bug touches allocator/mouse-decoder logic
- **On-OS scenarios** (`tests/thread_test`) — add a scenario (e.g. a new `src/*_test.rs`) when a bug touches syscall behavior
- **Kernel self-tests** (`self_test` feature) — add a `selftest::register(...)` TAP assertion for in-kernel invariants
- **QEMU boot tests** — the strongest regression gate is "system still boots to `login:` and passes kernel TAP"

## Suggested Workflow for a Fixed Bug

1. Write a test that reproduces the original bug (host suite, thread_test scenario, or selftest)
2. Verify the test fails without the fix
3. Reference the issue in the test name/comment
4. CI (`integration-qemu`) will gate regressions of that behavior on every boot

## Running

```bash
cargo run --manifest-path tests/skyos-test/Cargo.toml -- run
./tests/qemu_boot.sh
```

There is no `cargo test --test regression`; CI does not compare against previous-build results beyond the normal test pass/fail.
