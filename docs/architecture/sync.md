# Synchronization Primitives

SkyOS uses the `spin` crate's `Mutex`/`RwLock` throughout the kernel, plus futexes for userspace.

## Spinlocks

`spin::Mutex` is the primary kernel lock (`use spin::Mutex` in scheduler, process, futex, keyboard, VFS, etc.). Interrupt handlers use `try_lock` to avoid deadlocks.

```rust
// spin crate Mutex — the kernel does not define a custom Spinlock type.
```

## Futexes

Futexes (`syscalls/futex.rs`, `SYS_FUTEX`) provide a hybrid approach: the fast path is entirely in userspace using atomic compare-and-swap, and only contention triggers a syscall. This is the preferred synchronization mechanism for performance-sensitive userspace code.

## Other Notes

- **No sleeping mutexes**: the kernel uses spinlocks, not sleepable mutexes with wait queues.
- **No condvars, barriers, or semaphores** exist in the kernel.
- Blocking (sleep, pipe I/O, futex wait) is implemented via thread state + global sleep/block queues in the scheduler, not condvar primitives.
