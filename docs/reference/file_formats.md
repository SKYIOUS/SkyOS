# Filesystems Supported

This page lists the filesystem implementations in `kernel/kernel/src/vfs/`.

## Current Support

| Module | Kind | Operations |
|--------|------|------------|
| ramfs | In-memory | Full (read/write/create/delete) — mounted at `/tmp` |
| skyfs | On-disk | Full — default on-disk filesystem |
| tarfs | Read-only archive | Read — used for the initrd |
| devfs | Virtual | Device nodes |
| ctlfs | Virtual | Control interfaces (e.g. `/dev/stdin`) |
| pipe | Stream | In-memory pipe/stream pairs |
| ext2 | On-disk | Read/write |
| ext4 | On-disk | Read-only, behind the `ext4` feature flag |
| fat | On-disk | FAT32 read/write |

## Notes

- **skyfs**: Uses the VFS global page cache (`vfs/page_cache.rs`, `GLOBAL_PAGE_CACHE`).
- **tarfs**: Used to load the userspace initrd at boot (root filesystem unless a block device ext2/ext4 filesystem is found).
- **devfs**: Exposes kernel devices such as the framebuffer, serial ports, mouse/keyboard input, and `null`/`zero`/`random`.
- **Block caching**: storage drivers sit behind `drivers/block/cache.rs` (`BlockCache`), registered via `register_block_device`.
