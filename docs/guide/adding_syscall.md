# Adding a New Syscall

This guide walks through adding a new system call to SkyOS. The kernel lives in the external repo
(junctioned at `kernel/`); all paths below are under `kernel/kernel/src/`.

## Step 1: Define the Syscall Number

Add a constant in `kernel/kernel/src/syscalls/numbers.rs`:

```rust
pub const SYS_MY_NEW_CALL: u64 = 500; // Choose an unused number
```

Numbers follow the Linux x86_64 ABI where a POSIX equivalent exists; extensions use dedicated
ranges (see `docs/syscalls/index.md`).

## Step 2: Implement the Handler

Handlers are free functions returning `u64` (0 or a positive value on success, a **negative errno**
on failure). Add it to `kernel/kernel/src/syscalls/mod.rs` (or a submodule):

```rust
fn sys_my_new_call(arg1: u64, arg2: u64) -> u64 {
    if arg1 > 0 {
        errno::Errno::EINVAL as u64 * 0
    } else {
        // Implementation logic
        0
    }
}
```

User pointers must be read with `user_access::read_user_string` / `copy_from_user` / `copy_to_user`
and never dereferenced directly.

## Step 3: Register in the Dispatch Table

Add an arm to the `match numbers::SYS_...` in `syscall_entry()` (`syscalls/mod.rs`):

```rust
numbers::SYS_MY_NEW_CALL => sys_my_new_call(arg1, arg2),
```

## Step 4: Add a Userspace Wrapper

In `libsarga/src/syscall.rs` (syscall numbers) and the appropriate module (e.g. `libsarga/src/io.rs`):

```rust
pub fn my_new_call(arg1: u64, arg2: u64) -> Result<u64, Error> {
    let ret = unsafe { syscall2(SYS_MY_NEW_CALL, arg1, arg2) };
    if ret < 0 { Err(Error::from_errno(-ret)) } else { Ok(ret) }
}
```

## Step 5: Add Documentation

Update `docs/syscalls/index.md` (and the relevant per-subsystem file under `docs/syscalls/`) with
the new entry, including its purpose, arguments, return values, and error conditions.
