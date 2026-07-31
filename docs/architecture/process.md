# Process and Thread Model

SkyOS has distinct `Process` and `Thread` types.

## Structure

Processes are reference-counted `Arc<Process>` stored in `PROCESS_TABLE: Mutex<BTreeMap<u64, Arc<Process>>>`, with `CURRENT_PROCESS` tracking the running one:

```rust
pub struct Process {
    pid: u64,
    fd_table: Mutex<Vec<Option<FileDescriptor>>>,  // per-process
    fd_flags: ...,
    handle_table: HandleTable,
    vmas: Mutex<Vec<Vma>>,
    children: Vec<...>,
    brk: ...,
    exit_code: ...,
    credentials, capabilities, signal state, ...
}
```

`FileDescriptor` is an enum: `File { node, offset } | PtyMaster | PtySlave | EventFd`.

## Threads

`Thread { _id, stack, stack_ptr, status, process: Option<Arc<Process>>, priority, sleep_until, futex_wake_addr, pipe_block_key, fs_base, pass, stride, tickets }`. Each thread has its own kernel stack (with guard page) and is scheduled independently by the stride scheduler.

## Process Lifecycle

1. **Creation**: `fork()` clones the address space (copy-on-write via `clone_cow()`); `exec()` replaces it.
2. **Scheduling**: Threads run under the preemptive stride scheduler.
3. **Blocking**: Threads block on sleep (`sleep_until`), futex, pipe, or I/O.
4. **Termination**: `exit()` leaves a zombie until the parent `wait4()`s.

## Thread Local Storage

Each thread has TLS pointed to by the `FS` segment base. `read_fs_base()`/`write_fs_base()` use the `wrfsbase` instruction when available, else MSR `0xC0000100`. TLS is switched on context switches.
