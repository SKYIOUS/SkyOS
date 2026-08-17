# Userspace Isolation and Process Separation

SkyOS ensures that userspace processes cannot interfere with each other or the kernel.

## Address Space Isolation

Each process has an independent address space controlled by a separate page table. The kernel enforces:
- No process can access another process's memory
- No process can access kernel memory (except via syscalls)
- The kernel cannot accidentally access userspace memory (SMAP)

```rust
pub struct Process {
    id: ProcessId,
    address_space: AddressSpace,   // page table + VMA tracking
    // ...
}
```

Address spaces are switched via `address_space.activate()` during context switch / exec.

## File Descriptor Isolation

File descriptors are process-local. One process cannot access another process's file descriptors. When a file descriptor is shared (via fork or UNIX domain sockets), both processes have independent references to the same kernel object.

## Signal Isolation

Processes can only send signals to:
- Themselves
- Processes in the same process group
- Processes owned by the same user (or root)

Signals cannot be sent across user boundaries without appropriate capabilities.

## IPC Security

IPC mechanisms are subject to access controls:
- **Pipes**: Only accessible by processes that hold the file descriptor
- **Shared memory**: Protected by file permissions on the shm object
- **Message queues**: Protected by file permissions
- **UNIX sockets**: Protected by filesystem permissions

## Resource Limits

`getrlimit`/`setrlimit`/`prlimit64` syscalls (in `syscalls/mod.rs`) store per-process `rlim_cur`/`rlim_max` arrays (16 slots) with root-only privilege to raise `rlim_max`. **The limits are stored but not currently enforced** — nothing caps address space, fd count, or CPU time against them.

| Limit slot | Description |
|------------|-------------|
| 0 | CPU time (stored, unenforced) |
| ... | other POSIX slots (stored, unenforced) |

## Process State Transitions

Processes transition between states:
- **Running**: Executing on a CPU
- **Runnable**: Ready to execute, waiting for scheduler
- **Blocked**: Waiting for I/O, IPC, or timer
- **Zombie**: Terminated, waiting for parent to collect exit status
- **Dead**: Resources freed
