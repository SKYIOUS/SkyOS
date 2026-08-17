# Inter-process Communication

SkyOS provides multiple IPC mechanisms designed for different use cases.

## Sockets (AF_UNIX)

The primary IPC mechanism is **AF_UNIX sockets** (`net/unix.rs`). They provide full socket semantics: `bind`, `connect`, `listen`, `accept`, `sendto`/`recvfrom`, `sendmsg`/`recvmsg`. The ADE desktop uses one socketpair per spawned app for its request/response transport.

There is **no channel/RingBuffer/IPC-port subsystem** (`src/ipc/` does not exist in the kernel).

## Pipes

`vfs/pipe.rs` provides in-memory pipes exposed via `sys_pipe`, used for shell pipelines and stream IPC.

## Shared Memory

Processes can share memory regions via `mmap()` (COW fork via `clone_cow()`). 

## Signals

Signals provide notification between processes. Each process has a pending/blocked signal bitmask and a signal handler table (`syscalls/signal.rs`). The kernel delivers signals by setting the pending bit, waking the thread if blocked, and invoking handlers in the syscall postamble.

## Other Primitives

- **futex** (`SYS_FUTEX`): Userspace synchronization
- **eventfd**: Lightweight event notification file descriptor
- **PTY** (`pty.rs`): Terminal emulation (PtyMaster/PtySlave file descriptors)
