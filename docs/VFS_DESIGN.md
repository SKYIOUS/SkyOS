# SkyOS Virtual Filesystem (VFS) Design

This document outlines the design of the SkyOS Virtual Filesystem (VFS) layer, which provides a unified interface for user applications to interact with different filesystems and I/O devices.

## 1. Core Concepts

The VFS is built around two primary traits:

-   **`VfsNode`**: Represents an object in the filesystem tree, which can be a file, directory, device, or pipe.
-   **`FileSystem`**: Represents a mounted filesystem instance, responsible for providing the root `VfsNode` of its own tree.

A global `VFS` manager (a `pub static VFS: SchedLock<VfsManager>`) tracks all mounted filesystems and handles path resolution.

## 2. `VfsNode` Trait

The `VfsNode` trait is the core abstraction. All filesystem objects must implement this trait.

```rust
pub trait VfsNode: Send + Sync {
    fn name(&self) -> String;
    fn is_dir(&self) -> bool;
    fn read(&self, max_len: usize) -> Result<Vec<u8>, ()>;
    fn write(&self, data: &[u8]) -> Result<(), ()>;
    // optional methods with defaults: stat, statfs, ioctl, children,
    // find_child, create, mkdir, unlink, chmod, chown, readlink
}
```

(The `FileSystem` trait is `fn root(&self) -> Result<Arc<dyn VfsNode>, ()>`.)

## 3. Path Resolution

Path resolution starts at the root (`/`) and traverses the VFS tree.

1.  `VFS::init()` mounts the root: a block-device ext4/ext2 filesystem if one is found, otherwise the bootloader-provided initrd as tarfs (`/`).
2.  It splits the path into components (e.g., `/home/user/file` -> `home`, `user`, `file`).
3.  For each component, it checks if the current path is a mount point. If so, it switches to the root of the mounted filesystem.
4.  It calls the `children()` method on the current directory node and finds the node with the matching name.
5.  This process repeats until the final component is resolved.

## 4. Supported Filesystems

-   **`ramfs.rs`**: In-memory filesystem used for `/tmp`.
-   **`devfs.rs`**: Device filesystem mounted at `/dev`.
-   **`ctlfs.rs`**: Control filesystem mounted at `/ctl`.
-   **`pipe.rs`**: In-memory pipe for inter-process communication (IPC), exposed via `sys_pipe`.
-   **`tarfs.rs`**: Read-only tar archive filesystem (boot initrd payload, mounted at `/` unless a block device filesystem is found).
-   **`skyfs.rs`**: SkyOS native filesystem.
-   **`ext2.rs`**: Read-write Second Extended Filesystem (inode/block writes implemented).
-   **`ext4.rs`**: Ext4 read-only support (feature-gated; boot tries ext4 → ext2 on block devices).
-   **`fat.rs`**: Wrapper around the `fatfs` crate for FAT32-formatted devices.

## 5. File Descriptors

Each `Process` has its own file descriptor table (`fd_table`), which is a `Vec<Option<FileDescriptor>>`. A file descriptor can point to a `VfsNode` (for files) or a `Socket` handle. This per-process table allows for standard I/O redirection and inheritance across forks.
