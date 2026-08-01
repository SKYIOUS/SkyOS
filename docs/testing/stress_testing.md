# Stress and Stability Testing

There is no dedicated stress-test binary or CI stress schedule. Stability is exercised through:

## On-OS Scenarios (`tests/thread_test`)

`thread_test` is a `#![no_std]` userspace crate that logs in and runs syscall-heavy scenarios on real hardware:

- **futex_test.rs**: futex wait/wake, fork, sched_setattr, yield
- **dac_test.rs** / **perm_test.rs**: permission checks against real credentials
- **pipe_signal_test.rs**: pipe + signal interplay
- **sigalrm/sigchld/sigint_test.rs**: signal delivery to real processes

Run it by booting to the login prompt and executing the `thread_test` binary.

## Memory Stress

The buddy allocator is stressed host-side in `tests/skyos-test-core/src/suites/kernel_alloc.rs` (allocation, free, buddy merging, fragmentation, exhaustion, merge chains) — a reimplementation of the kernel algorithm, not the live allocator.

## Boot Loop Stability

`tests/qemu_boot.sh` and `tests/qemu_integration_test.sh` can be run repeatedly to shake out boot-time races. Kernel `self_test` TAP assertions (allocator/FS/net invariants) run on every boot.

## File System Stress

No dedicated FS stress suite exists. FS behavior is exercised through the boot/initrd load (`tarfs`) and the QEMU boot-to-login path.
