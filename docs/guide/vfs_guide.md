# Working with the Virtual File System

The Virtual File System (VFS) provides a unified interface for different filesystem implementations.

## VFS Architecture

The VFS uses a node-based tree where every filesystem object (file, directory, device, mount point) is an `Arc<dyn VfsNode>` (`kernel/kernel/src/vfs/mod.rs`). The trait is intentionally small:

```rust
pub trait VfsNode: Send + Sync {
    fn read(&self, max_len: usize) -> Result<Vec<u8>, ()>;
    fn write(&self, data: &[u8]) -> Result<(), ()>;
}
```

Most concrete nodes additionally expose `lookup`/`create`/`as_any` downcast helpers for directory operations, but the generic contract is `read`/`write`. File descriptors carry the node plus an offset; `read`/`write` return the full buffer rather than a cursor API (seek state lives in the `FileDescriptor`, see `task/process.rs`).

## VFS Manager

The manager is a global `pub static VFS: SchedLock<VfsManager>` at `vfs/mod.rs:388`. Mounting happens through the manager rather than a free function:

```rust
VFS.lock().mount("/", filesystem); // filesystem: Arc<dyn FileSystem>
```

## File Operations

Standard file operations go through the syscall layer, which dispatches to VFS nodes:
- `open()` → path resolution + file descriptor allocation (`File { node, offset }`)
- `read()`/`write()` → node `read`/`write` at the descriptor's offset
- `close()` → descriptor release

## Supported Filesystems

Mountable filesystems (`kernel/kernel/src/vfs/`):

- **ramfs**: In-memory filesystem mounted at `/tmp`
- **skyfs**: Default on-disk filesystem
- **ext2**: On-disk filesystem (read/write)
- **ext4**: Read-only support behind the `ext4` feature flag
- **fat**: FAT32 read/write
- **tarfs**: Read-only TAR archive filesystem (initrd; root at boot unless a block device filesystem is found)
- **devfs**: Virtual filesystem exposing device nodes
- **pipe**: Pipe/stream pseudo-filesystem
- **ctlfs**: Control interfaces (e.g. `/dev/stdin`)

## Node Conventions

- Read returns a fresh `Vec<u8>`; length `0` signals EOF/end-of-stream for files.
- `write` appends/consumes per the node's semantics (files overwrite at the current descriptor offset; pipes append to the queue).
- Errors are `()` — the syscall layer maps them to a negative errno.
