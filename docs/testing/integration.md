# Testing SkyOS

SkyOS testing is split across host-side unit tests, on-OS integration binaries, and QEMU boot/integration scripts.

## Host-Side Test Framework (`tests/skyos-test`)

`tests/skyos-test-core` is a small host-side framework that registers `Test { name, category, run: Fn() -> Result<(), String> }` entries and runs them with `skyos-test-core::TestRunner`. Suites live in `tests/skyos-test-core/src/suites/`:

- `kernel_alloc` — a host-side reimplementation of the kernel buddy allocator algorithm (allocate/free/merge/fragmentation/exhaustion)
- `kernel_mouse` — host-side validation of the mouse packet decoder

The `skyos-test` CLI (`tests/skyos-test`) drives them:

```bash
cargo run --manifest-path tests/skyos-test/Cargo.toml -- list
cargo run --manifest-path tests/skyos-test/Cargo.toml -- run                 # console
cargo run --manifest-path tests/skyos-test/Cargo.toml -- run --format json --output results.json
cargo run --manifest-path tests/skyos-test/Cargo.toml -- run --category kernel::alloc
cargo run --manifest-path tests/skyos-test/Cargo.toml -- report results.json -o report.html
```

This is host-side only — it validates algorithms against a mock, not the running kernel.

## On-OS Integration Binaries (`tests/thread_test`)

`tests/thread_test` is a `#![no_std]` `#![no_main]` userspace crate that boots to the login prompt, logs in, and exercises real kernel syscalls. Each `src/*_test.rs` is a scenario: futex, DAC/perm, pipe+signal, sigalrm, sigchld, sigint. Tests use `libsarga::sarga_main!` and raw `libsarga::syscall::syscallN` wrappers.

## QEMU Boot Smoke Test (`tests/qemu_boot.sh`)

`./tests/qemu_boot.sh [kernel_dir]` (kernel defaults to `../SKYIOUS KERNEL`):

1. Builds the kernel (`--release --target x86_64-unknown-none -Zbuild-std=core,alloc --features net,smp,ai_rule`)
2. Builds userspace (`--release --target x86_64-sarga.json`)
3. Builds the initrd (`build_initrd.py`, copied to `<kernel_dir>/SkyOS/`)
4. Creates the UEFI bootimage (`cargo run --release --manifest-path kernel/builder/Cargo.toml`)
5. Creates an ISO (`scripts/make_iso.py`)
6. Boots in QEMU (OVMF, 512M, 2 cpus, e1000) with a 120s timeout
7. PASS on a `login:` prompt; FAIL on `panic` or timeout

## QEMU Integration Test (`tests/qemu_integration_test.sh`)

Same pipeline but runs `expect` (`tests/qemu_shell_test.exp`) for automated interaction with the shell after boot. Falls back to a plain boot-to-login check when `expect` isn't installed.

## Windows PowerShell Boot Tests (`tests/*.ps1`)

`test_boot.ps1`, `test_login.ps1`, `test_panic.ps1` are PowerShell equivalents of the QEMU boot/login/panic smoke tests.

## Kernel Self-Test Feature

The kernel `self_test` feature emits TAP-format results (`TAP version 13`, `ok`/`not ok`) to serial on boot. CI's `integration-qemu` job scans the boot log for `ok`/`not ok` lines and fails on any `not ok`, so kernel-level unit tests gate merges from inside QEMU.

## What the Tests Cover

- **Kernel allocator**: buddy allocate/free/merge logic (host-side mock)
- **Syscalls**: futex, sched_setattr, fork/exit/wait, pipe+signal interplay (thread_test)
- **DAC/perms**: `perm_test.rs`, `dac_test.rs` exercise permission checks against real credentials
- **Signals**: sigalrm, sigchld, sigint delivery to real processes
- **Boot integrity**: kernel boots cleanly to a `login:` prompt without panicking

## Running Everything

```bash
# host-side suites
cargo run --manifest-path tests/skyos-test/Cargo.toml -- run

# QEMU boot smoke test (kernel dir defaults to ../SKYIOUS KERNEL)
./tests/qemu_boot.sh

# Full integration with expect-driven shell interaction
./tests/qemu_integration_test.sh
```

## Test Configuration

QEMU is run headless: `-nographic`, `-serial mon:stdio`, `-no-reboot`, 512M RAM, 2 cpus, an e1000 NIC (user-mode networking). Logs go to a temp file for `grep`-based pass/fail assertions.
