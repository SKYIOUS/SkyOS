//! File and stream I/O operations.

use crate::errno::Error;
use alloc::string::String;
use alloc::vec::Vec;
use crate::syscall::*;

/// File metadata structure.
#[derive(Debug, Clone, Copy)]
pub struct Stat {
    /// Device ID
    pub dev: u64,
    /// Inode number
    pub ino: u64,
    /// Number of hard links
    pub nlink: u64,
    /// File mode and permissions
    pub mode: u32,
    /// User ID of owner
    pub uid: u32,
    /// Group ID of owner
    pub gid: u32,
    /// Total size in bytes
    pub size: u64,
    /// Block size for filesystem I/O
    pub blksize: u64,
}

impl Stat {
    fn from_bytes(buf: &[u8]) -> Self {
        use core::convert::TryInto;
        let g = |o: usize| -> u64 { u64::from_ne_bytes(buf[o..o+8].try_into().unwrap_or([0;8])) };
        let w = |o: usize| -> u32 { u32::from_ne_bytes(buf[o..o+4].try_into().unwrap_or([0;4])) };
        Stat { dev: g(0), ino: g(8), mode: w(16), nlink: g(24), uid: w(32), gid: w(36), size: g(48), blksize: g(64) }
    }
}

/// Opens a file at the given path with specified flags.
pub fn open(path: &str, flags: i32) -> Result<i64, Error> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(Error::EINVAL); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    // SAFETY: open syscall is safe here
    let r = unsafe { crate::syscall::open(buf.as_ptr(), flags) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r) }
}

/// Reads from a file descriptor into a buffer.
pub fn read(fd: i64, buf: &mut [u8]) -> Result<usize, Error> {
    // SAFETY: read syscall is safe here
    let r = unsafe { crate::syscall::read(fd, buf.as_mut_ptr(), buf.len()) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r as usize) }
}

/// Writes from a buffer to a file descriptor.
pub fn write(fd: i64, buf: &[u8]) -> Result<usize, Error> {
    // SAFETY: write syscall is safe here
    let r = unsafe { crate::syscall::write(fd, buf.as_ptr(), buf.len()) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r as usize) }
}

/// Writes all bytes in a buffer to a file descriptor, retrying if necessary.
pub fn write_all(fd: i64, mut buf: &[u8]) -> Result<(), Error> {
    while !buf.is_empty() {
        let n = write(fd, buf)?;
        if n == 0 { return Err(Error::EIO); }
        buf = &buf[n..];
    }
    Ok(())
}

/// Closes a file descriptor.
pub fn close(fd: i64) -> Result<(), Error> {
    // SAFETY: close syscall is safe here
    let r = unsafe { crate::syscall::close(fd) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Retrieves file metadata for a given path.
pub fn stat(path: &str) -> Result<Stat, Error> {
    let mut buf = [0u8; 144];
    let mut path_buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(Error::EINVAL); }
    path_buf[..bytes.len()].copy_from_slice(bytes);
    path_buf[bytes.len()] = 0;
    // SAFETY: stat syscall is safe here
    let r = unsafe { crate::syscall::syscall3(SYS_STAT, path_buf.as_ptr() as u64, buf.as_mut_ptr() as u64, 0) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(Stat::from_bytes(&buf)) }
}

/// Retrieves file metadata for an open file descriptor.
pub fn fstat(fd: i64, buf: &mut Stat) -> Result<(), Error> {
    let mut raw_buf = [0u8; 144];
    // SAFETY: fstat syscall is safe here
    let r = unsafe { crate::syscall::fstat(fd, raw_buf.as_mut_ptr()) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        *buf = Stat::from_bytes(&raw_buf);
        Ok(())
    }
}

/// Creates a new directory.
pub fn mkdir(path: &str, mode: u32) -> Result<(), Error> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(Error::EINVAL); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    // SAFETY: mkdir syscall is safe here
    let r = unsafe { crate::syscall::mkdir(buf.as_ptr(), mode) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Deletes a file.
pub fn unlink(path: &str) -> Result<(), Error> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(Error::EINVAL); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    // SAFETY: unlink syscall is safe here
    let r = unsafe { crate::syscall::unlink(buf.as_ptr()) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Reads an entire file into a String.
pub fn read_to_string(path: &str) -> Result<String, Error> {
    let fd = open(path, 0)?;
    let mut buf = [0u8; 4096];
    let mut result = String::new();
    loop {
        match read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                    result.push_str(s);
                }
            }
            Err(e) => { let _ = close(fd); return Err(e); }
        }
    }
    let _ = close(fd);
    Ok(result)
}

/// Prints a string to standard output.
pub fn print_str(s: &str) {
    let _ = write_all(1, s.as_bytes());
}

/// Retrieves the current working directory.
pub fn getcwd(buf: &mut [u8]) -> Result<usize, Error> {
    // SAFETY: getcwd syscall is safe here
    let r = unsafe { crate::syscall::syscall2(SYS_GETCWD, buf.as_mut_ptr() as u64, buf.len() as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r as usize) }
}

/// Changes the current working directory.
pub fn chdir(path: &str) -> Result<(), Error> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(Error::EINVAL); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    // SAFETY: chdir syscall is safe here
    let r = unsafe { crate::syscall::syscall1(SYS_CHDIR, buf.as_ptr() as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Reads directory entries from an open file descriptor.
pub fn getdents64(fd: i64, buf: &mut [u8]) -> Result<usize, Error> {
    // SAFETY: getdents64 syscall is safe here
    let r = unsafe { crate::syscall::getdents64(fd, buf.as_mut_ptr(), buf.len()) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r as usize) }
}

/// Sleeps the current thread for a given number of nanoseconds.
pub fn nanosleep(ns: u64) -> Result<(), Error> {
    // SAFETY: nanosleep syscall is safe here
    let r = unsafe { crate::syscall::syscall2(SYS_NANOSLEEP, ns, 0) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Flushes filesystem buffers to disk.
pub fn sync() -> i64 {
    // SAFETY: sync syscall is safe here
    unsafe { crate::syscall::syscall0(36) }
}

/// Reboots or powers off the system.
pub fn reboot() -> i64 {
    // magic=0xDEAD_BEEF, cmd=0 (poweroff) or 1 (reboot)
    // SAFETY: reboot syscall is safe here
    unsafe { crate::syscall::syscall2(169, 0xDEAD_BEEF, 0) }
}

/// Changes permissions of an open file.
pub fn fchmod(fd: u64, mode: u32) -> Result<(), Error> {
    // SAFETY: fchmod syscall is safe here
    let r = unsafe { crate::syscall::syscall2(SYS_FCHMOD, fd, mode as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Changes ownership of an open file.
pub fn fchown(fd: u64, uid: u32, gid: u32) -> Result<(), Error> {
    // SAFETY: fchown syscall is safe here
    let r = unsafe { crate::syscall::syscall3(SYS_FCHOWN, fd, uid as u64, gid as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Mouse state information.
#[derive(Debug, Clone, Copy)]
pub struct MouseState {
    /// X coordinate
    pub x: u64,
    /// Y coordinate
    pub y: u64,
    /// Button bitmask
    pub buttons: u8,
    /// Scroll delta
    pub scroll: i8,
}

/// Retrieves the current mouse state for a window.
pub fn get_mouse(handle: u64) -> MouseState {
    // SAFETY: gui get_mouse syscall is safe here
    let packed = unsafe { crate::syscall::syscall1(120, handle) };
    if packed < 0 {
        MouseState { x: 0, y: 0, buttons: 0, scroll: 0 }
    } else {
        MouseState {
            x: packed as u64 & 0xFFFF,
            y: (packed as u64 >> 16) & 0xFFFF,
            buttons: ((packed as u64 >> 32) & 0xFF) as u8,
            scroll: ((packed as u64 >> 40) & 0xFF) as i8,
        }
    }
}

/// Sets the title of a window.
pub fn set_title(handle: u64, title: &str) {
    let mut buf = [0u8; 65];
    let bytes = title.as_bytes();
    let len = bytes.len().min(64);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
    // SAFETY: gui set_title syscall is safe here
    unsafe { crate::syscall::syscall2(121, handle, buf.as_ptr() as u64); }
}

/// Destroys a window.
pub fn destroy_window(handle: u64) {
    // SAFETY: gui destroy_window syscall is safe here
    unsafe { crate::syscall::syscall1(122, handle); }
}

/// Resizes a window.
pub fn resize_window(handle: u64, width: u64, height: u64) {
    // SAFETY: gui resize_window syscall is safe here
    unsafe { crate::syscall::syscall3(123, handle, width, height); }
}

/// Moves a window to a new location.
pub fn move_window(handle: u64, x: u64, y: u64) {
    // SAFETY: gui move_window syscall is safe here
    unsafe { crate::syscall::syscall3(124, handle, x, y); }
}

/// Reads data from the system clipboard.
pub fn clipboard_read(buf: &mut [u8]) -> usize {
    // SAFETY: clipboard_read syscall is safe here
    let r = unsafe { crate::syscall::syscall3(125, 0, buf.as_mut_ptr() as u64, buf.len() as u64) };
    if r < 0 { 0 } else { r as usize }
}

/// Writes data to the system clipboard.
pub fn clipboard_write(data: &[u8]) {
    // SAFETY: clipboard_write syscall is safe here
    unsafe { crate::syscall::syscall3(125, 1, data.as_ptr() as u64, data.len() as u64); }
}

/// Retrieves the length of data currently in the clipboard.
pub fn clipboard_len() -> usize {
    // SAFETY: clipboard_len syscall is safe here
    let r = unsafe { crate::syscall::syscall1(125, 2) };
    if r < 0 { 0 } else { r as usize }
}

/// Sends a system notification.
pub fn notify(text: &str, duration_ms: u64) {
    let mut buf = [0u8; 257];
    let bytes = text.as_bytes();
    let len = bytes.len().min(256);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
    // SAFETY: notify syscall is safe here
    unsafe { crate::syscall::syscall3(126, buf.as_ptr() as u64, duration_ms, 0); }
}

/// Creates a pseudo-terminal pair.
pub fn openpty() -> Result<(i64, i64), Error> {
    // SAFETY: openpty syscall is safe here
    let r = unsafe { crate::syscall::syscall0(210) };
    if r < 0 { return Err(Error::from_i64(r)); }
    Ok(((r >> 32) as i64, (r & 0xFFFF_FFFF) as i64))
}

/// Duplicates a file descriptor.
pub fn dup2(oldfd: i64, newfd: i64) -> Result<(), Error> {
    // SAFETY: dup2 syscall is safe here
    let r = unsafe { crate::syscall::syscall2(SYS_DUP2, oldfd as u64, newfd as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Mount a filesystem.
pub fn mount(source: &str, target: &str, fstype: &str, flags: u64) -> Result<(), Error> {
    let mut src = Vec::from(source.as_bytes()); src.push(0);
    let mut tgt = Vec::from(target.as_bytes()); tgt.push(0);
    let mut fs = Vec::from(fstype.as_bytes()); fs.push(0);
    // SAFETY: mount syscall is safe here
    let r = unsafe { crate::syscall::syscall6(SYS_MOUNT, src.as_ptr() as u64, tgt.as_ptr() as u64, fs.as_ptr() as u64, flags, 0, 0) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Unmount a filesystem.
pub fn umount(target: &str) -> Result<(), Error> {
    let mut tgt = Vec::from(target.as_bytes()); tgt.push(0);
    // SAFETY: umount syscall is safe here
    let r = unsafe { crate::syscall::syscall2(SYS_UMOUNT2, tgt.as_ptr() as u64, 0) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// File descriptor set for select().
#[repr(C)]
pub struct FdSet {
    pub bits: [u64; 16], // Up to 1024 FDs
}

impl FdSet {
    pub fn new() -> Self {
        Self { bits: [0; 16] }
    }

    pub fn set(&mut self, fd: i64) {
        if fd >= 0 && fd < 1024 {
            self.bits[(fd / 64) as usize] |= 1 << (fd % 64);
        }
    }

    pub fn clear(&mut self, fd: i64) {
        if fd >= 0 && fd < 1024 {
            self.bits[(fd / 64) as usize] &= !(1 << (fd % 64));
        }
    }

    pub fn is_set(&self, fd: i64) -> bool {
        if fd >= 0 && fd < 1024 {
            (self.bits[(fd / 64) as usize] & (1 << (fd % 64))) != 0
        } else {
            false
        }
    }
}

/// Monitor multiple file descriptors.
pub fn select(nfds: i32, readfds: Option<&mut FdSet>, writefds: Option<&mut FdSet>, exceptfds: Option<&mut FdSet>, timeout_ms: Option<i32>) -> Result<i32, Error> {
    let r = unsafe {
        crate::syscall::syscall5(
            23, // SYS_SELECT
            nfds as u64,
            readfds.map_or(0, |f| f as *mut _ as u64),
            writefds.map_or(0, |f| f as *mut _ as u64),
            exceptfds.map_or(0, |f| f as *mut _ as u64),
            timeout_ms.map_or(0, |t| t as u64),
        )
    };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r as i32) }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        let s = $crate::alloc::format!($($arg)*);
        $crate::io::print_str(&s);
    }}
}
#[macro_export]
macro_rules! println {
    () => { $crate::io::print_str("\n") };
    ($($arg:tt)*) => {{
        let s = $crate::alloc::format!($($arg)*);
        $crate::io::print_str(&s);
        $crate::io::print_str("\n");
    }}
}
