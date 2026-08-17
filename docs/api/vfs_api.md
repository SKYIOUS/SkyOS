# VFS API Reference

The Virtual File System API provides the interface for filesystem implementations and file
operations. The two central traits are `VfsNode` and `FileSystem`, managed by a global
`pub static VFS: SchedLock<VfsManager>`.

## VfsNode Trait

Every filesystem object (file, directory, device, pipe) implements `VfsNode`. All methods have
default implementations that return `Err(())` unless overridden:

```rust
pub trait VfsNode: Send + Sync {
    fn name(&self) -> String;
    fn is_dir(&self) -> bool;
    fn read(&self, max_len: usize) -> Result<Vec<u8>, ()>;

    fn inode_num(&self) -> Option<u64>;                    // hard links
    fn stat(&self) -> Result<Stat, ()>;
    fn statfs(&self) -> Result<StatFs, ()>;
    fn write(&self, data: &[u8]) -> Result<(), ()>;
    fn ioctl(&self, request: u64, argp: *mut u8) -> Result<u64, ()>;
    fn children(&self) -> Result<Vec<Arc<dyn VfsNode>>, ()>;
    fn find_child(&self, name: &str) -> Option<Arc<dyn VfsNode>>;   // default: scan children()
    fn mkdir(&self, name: &str) -> Result<Arc<dyn VfsNode>, ()>;
    fn create(&self, name: &str) -> Result<Arc<dyn VfsNode>, ()>;
    fn unlink(&self, name: &str) -> Result<(), ()>;
    fn chmod(&self, mode: u32) -> Result<(), ()>;
    fn chown(&self, uid: u32, gid: u32) -> Result<(), ()>;
    fn readlink(&self) -> Result<String, ()>;
    fn symlink(&self, name: &str, target: &str) -> Result<(), ()>;
    fn rename(&self, old_name: &str, new_name: &str) -> Result<(), ()>;
    fn truncate(&self, len: i64) -> Result<(), ()>;
    fn link(&self, existing: Arc<dyn VfsNode>, name: &str) -> Result<(), ()>;
    fn utimens(&self, atime: (i64, i64), mtime: (i64, i64)) -> Result<(), ()>;
    fn fallocate(&self, mode: i32, offset: i64, len: i64) -> Result<(), ()>;
}
```

Errors use `Result<_, ()>` rather than an errno type at the trait boundary; syscalls translate these
into errnos.

## File Descriptors

A process's fd table maps integer fds to a `FileDescriptor` enum (not a struct):

```rust
pub enum FileDescriptor {
    File { node: Arc<dyn VfsNode>, offset: spin::Mutex<usize> },
    Socket(SocketHandle, SocketType),
    UnixSocket(u64, SocketType),
    PtyMaster { _idx: usize, pair: Arc<Mutex<PtyPair>> },
    PtySlave { _idx: usize, pair: Arc<Mutex<PtyPair>> },
    SignalFd(u64),
    EventFd(Arc<Mutex<EventFdData>>),
}
```

## Mounting

`VfsManager::mount(path, fs)` inserts a `MountPoint` (path + `Arc<dyn FileSystem>`); path
resolution checks mounted filesystems via `statfs_mount` (longest-prefix match, cached). The
`mount`/`umount2` syscalls (`docs/syscalls/index.md`) front this.

## Path Resolution

Paths are resolved from the root filesystem (tarfs over the bootloader initrd unless a block
device filesystem is mounted at `/`). Each component is looked up through
`children()`/`find_child()`, switching to a mounted filesystem root at mount points. Symlinks are
followed; `.` and `..` are handled during resolution.
