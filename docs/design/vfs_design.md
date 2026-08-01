# Virtual File System Design Decisions

The VFS layer provides a unified interface for diverse filesystem implementations.

## Node-Based Architecture

The VFS represents all filesystem objects as nodes in a tree. Each node is an `Arc<dyn VfsNode>` (`kernel/kernel/src/vfs/mod.rs`) that can be a file, directory, symlink, device, or mount point. This uniform representation simplifies path resolution and traversal.

The core contract is small — `name`, `is_dir`, `read(&self, max_len) -> Result<Vec<u8>, ()>`, `write(&self, data) -> Result<(), ()>` — with optional overrides for `stat`, `statfs`, `ioctl`, `children`, `find_child`, `mkdir`, `create`, `unlink`, `chmod`, `chown`, and `readlink`. Directory operations default to `Err(())` unless the filesystem implements them.

## Mount Hierarchy

A filesystem is mounted at a path via the global manager:

```rust
VFS.lock().mount(path, fs); // fs: Arc<dyn FileSystem>, root() -> Arc<dyn VfsNode>
```

`VFS` is a `pub static VFS: SchedLock<VfsManager>` (vfs/mod.rs). The manager holds the mount table; mounting replaces the node at `path` with the mounted filesystem's root.

## I/O Model

`read`/`write` are synchronous and return `Vec<u8>` / `()` — there is no async `WouldBlock` signal in the VFS contract. Non-blocking semantics live in the syscall layer (poll/read return EAGAIN/EWOULDBLOCK where appropriate). An empty read (`Vec::len() == 0`) signals EOF for regular files.

## Design Decisions

1. **Caching is explicit**: The VFS exposes a global page cache (`vfs/page_cache.rs`, `GLOBAL_PAGE_CACHE`) mapping `(inode_id, page_index)` to pages, with FIFO eviction; `skyfs` uses it. Block-level caching is separately provided by `drivers/block/cache.rs` (`BlockCache`). Filesystems opt in per-node.

2. **Synchronous node operations**: The trait is intentionally minimal; blocking behavior is pushed up to the syscall layer.

3. **File descriptor table is per-process and dynamic**: `fd_table: Mutex<Vec<Option<FileDescriptor>>>` in `task/process.rs` — descriptors are `FileDescriptor` enum variants (`File { node, offset }`, `Socket`, `UnixSocket`, `PtyMaster`/`PtySlave`, `SignalFd`, `EventFd`), not raw integers or a fixed-size array.

## Future Extensions

- FUSE (Filesystem in Userspace) support for userspace filesystems
- Stackable filesystem layers (encryption, compression, union mounts)
- Distributed filesystem support through networked VFS
