use crate::syscall::*;
pub const SYS_READ: u64 = 0; pub const SYS_WRITE: u64 = 1;
pub const SYS_OPEN: u64 = 2; pub const SYS_CLOSE: u64 = 3;
pub const SYS_LSEEK: u64 = 8;
pub const SYS_MOUNT: u64 = 165;
pub const SYS_UMOUNT2: u64 = 167;

pub fn open(path: &str, flags: i32) -> Result<i64, i64> {
    // Use a fixed stack buffer to avoid heap allocation issues
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let r = unsafe { syscall2(SYS_OPEN, buf.as_ptr() as u64, flags as u64) };
    if r < 0 { Err(-r) } else { Ok(r) }
}
pub fn read(fd: i64, buf: &mut [u8]) -> Result<usize, i64> {
    let r = unsafe { syscall3(SYS_READ, fd as u64, buf.as_mut_ptr() as u64, buf.len() as u64) };
    if r < 0 { Err(-r) } else { Ok(r as usize) }
}
pub fn write(fd: i64, buf: &[u8]) -> Result<usize, i64> {
    let r = unsafe { syscall3(SYS_WRITE, fd as u64, buf.as_ptr() as u64, buf.len() as u64) };
    if r < 0 { Err(-r) } else { Ok(r as usize) }
}
pub fn write_all(fd: i64, mut buf: &[u8]) -> Result<(), i64> {
    while !buf.is_empty() { let n = write(fd, buf)?; buf = &buf[n..]; }
    Ok(())
}
pub fn close(fd: i64) -> Result<(), i64> {
    let r = unsafe { syscall1(SYS_CLOSE, fd as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}
pub fn mount(source: &str, target: &str, fstype: &str, flags: u64) -> Result<(), i64> {
    let mut src = alloc::vec::Vec::from(source.as_bytes()); src.push(0);
    let mut tgt = alloc::vec::Vec::from(target.as_bytes()); tgt.push(0);
    let mut fs = alloc::vec::Vec::from(fstype.as_bytes()); fs.push(0);
    let r = unsafe { syscall6(SYS_MOUNT, src.as_ptr() as u64, tgt.as_ptr() as u64, fs.as_ptr() as u64, flags, 0, 0) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn umount(target: &str) -> Result<(), i64> {
    let mut tgt = alloc::vec::Vec::from(target.as_bytes()); tgt.push(0);
    let r = unsafe { syscall2(SYS_UMOUNT2, tgt.as_ptr() as u64, 0) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn sync() -> i64 {
    unsafe { crate::syscall::syscall0(36) }
}

pub fn reboot() -> i64 {
    // magic=0xDEAD_BEEF, cmd=0 (poweroff) or 1 (reboot)
    unsafe { crate::syscall::syscall2(169, 0xDEAD_BEEF, 0) }
}

pub fn dup2(oldfd: i64, newfd: i64) -> i64 {
    unsafe { crate::syscall::syscall2(33, oldfd as u64, newfd as u64) }
}

pub fn pipe() -> Result<(i64, i64), i64> {
    let mut fds = [0i64; 2];
    let r = unsafe { crate::syscall::syscall1(22, fds.as_mut_ptr() as u64) };
    if r < 0 { Err(-r) } else { Ok((fds[0], fds[1])) }
}

pub fn openpty() -> Result<(i64, i64), i64> {
    let r = unsafe { crate::syscall::syscall0(210) };
    if r < 0 { return Err(-r); }
    Ok(((r >> 32) as i64, (r & 0xFFFF_FFFF) as i64))
}

pub fn mkdir(path: &str, mode: u32) -> i64 {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return -crate::errno::EINVAL as i64; }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    unsafe { crate::syscall::syscall2(83, buf.as_ptr() as u64, mode as u64) }
}

pub fn fchmod(fd: u64, mode: u32) -> i64 {
    unsafe { syscall2(91, fd, mode as u64) }
}

pub fn fchown(fd: u64, uid: u32, gid: u32) -> i64 {
    unsafe { syscall3(93, fd, uid as u64, gid as u64) }
}

pub fn getdents64(fd: i64, buf: &mut [u8]) -> Result<usize, i64> {
    let r = unsafe { syscall3(217, fd as u64, buf.as_mut_ptr() as u64, buf.len() as u64) };
    if r < 0 { Err(-r) } else { Ok(r as usize) }
}

pub fn getcwd(buf: &mut [u8]) -> Result<i64, i64> {
    let r = unsafe { syscall2(79, buf.as_mut_ptr() as u64, buf.len() as u64) };
    if r < 0 { Err(-r) } else { Ok(r) }
}

pub fn chdir(path: &str) -> Result<(), i64> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let r = unsafe { syscall1(80, buf.as_ptr() as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn nanosleep(ns: u64) -> Result<(), i64> {
    let r = unsafe { syscall2(35, ns, 0) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn rmdir(path: &str) -> Result<(), i64> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let r = unsafe { syscall1(84, buf.as_ptr() as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn rename(old: &str, new: &str) -> Result<(), i64> {
    let mut old_buf = [0u8; 256];
    let mut new_buf = [0u8; 256];
    let old_bytes = old.as_bytes();
    let new_bytes = new.as_bytes();
    if old_bytes.len() > 254 || new_bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    old_buf[..old_bytes.len()].copy_from_slice(old_bytes);
    old_buf[old_bytes.len()] = 0;
    new_buf[..new_bytes.len()].copy_from_slice(new_bytes);
    new_buf[new_bytes.len()] = 0;
    let r = unsafe { syscall2(82, old_buf.as_ptr() as u64, new_buf.as_ptr() as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn unlink(path: &str) -> Result<(), i64> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let r = unsafe { syscall1(87, buf.as_ptr() as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn read_to_string(path: &str) -> Result<alloc::string::String, i64> {
    let fd = open(path, 0)?;
    let mut buf = alloc::vec![0u8; 4096];
    let mut result = alloc::string::String::new();
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

pub fn stat(path: &str) -> Result<Stat, i64> {
    let mut buf = [0u8; 144];
    let mut path_buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    path_buf[..bytes.len()].copy_from_slice(bytes);
    path_buf[bytes.len()] = 0;
    let r = unsafe { syscall3(4, path_buf.as_ptr() as u64, buf.as_mut_ptr() as u64, 0) };
    if r < 0 { Err(-r) } else { Ok(Stat::from_bytes(&buf)) }
}

pub struct Stat {
    pub dev: u64, pub ino: u64, pub nlink: u64,
    pub mode: u32, pub uid: u32, pub gid: u32,
    pub size: u64, pub blksize: u64,
}

impl Stat {
    fn from_bytes(buf: &[u8]) -> Self {
        use core::convert::TryInto;
        let g = |o: usize| -> u64 { u64::from_ne_bytes(buf[o..o+8].try_into().unwrap_or([0;8])) };
        let w = |o: usize| -> u32 { u32::from_ne_bytes(buf[o..o+4].try_into().unwrap_or([0;4])) };
        Stat { dev: g(0), ino: g(8), mode: w(16), nlink: g(24), uid: w(32), gid: w(36), size: g(48), blksize: g(64) }
    }
}

pub fn print_str(s: &str) { let _ = write_all(1, s.as_bytes()); }

// GUI Extended Syscalls
const SYS_GUI_GET_MOUSE: u64 = 120;
const SYS_GUI_SET_TITLE: u64 = 121;
const SYS_GUI_DESTROY_WINDOW: u64 = 122;
const SYS_GUI_RESIZE_WINDOW: u64 = 123;
const SYS_GUI_MOVE_WINDOW: u64 = 124;
const SYS_CLIPBOARD: u64 = 125;
const SYS_NOTIFY: u64 = 126;

pub struct MouseState {
    pub x: u64,
    pub y: u64,
    pub buttons: u8,
    pub scroll: i8,
}

pub fn get_mouse(handle: u64) -> MouseState {
    let packed = unsafe { crate::syscall::syscall1(SYS_GUI_GET_MOUSE, handle) };
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

pub fn set_title(handle: u64, title: &str) {
    let mut buf = [0u8; 65];
    let bytes = title.as_bytes();
    let len = bytes.len().min(64);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
    unsafe { crate::syscall::syscall2(SYS_GUI_SET_TITLE, handle, buf.as_ptr() as u64); }
}

pub fn destroy_window(handle: u64) {
    unsafe { crate::syscall::syscall1(SYS_GUI_DESTROY_WINDOW, handle); }
}

pub fn resize_window(handle: u64, width: u64, height: u64) {
    unsafe { crate::syscall::syscall3(SYS_GUI_RESIZE_WINDOW, handle, width, height); }
}

pub fn move_window(handle: u64, x: u64, y: u64) {
    unsafe { crate::syscall::syscall3(SYS_GUI_MOVE_WINDOW, handle, x, y); }
}

pub fn clipboard_read(buf: &mut [u8]) -> usize {
    let r = unsafe { crate::syscall::syscall3(SYS_CLIPBOARD, 0, buf.as_mut_ptr() as u64, buf.len() as u64) };
    if r < 0 { 0 } else { r as usize }
}

pub fn clipboard_write(data: &[u8]) {
    unsafe { crate::syscall::syscall3(SYS_CLIPBOARD, 1, data.as_ptr() as u64, data.len() as u64); }
}

pub fn clipboard_len() -> usize {
    let r = unsafe { crate::syscall::syscall1(SYS_CLIPBOARD, 2) };
    if r < 0 { 0 } else { r as usize }
}

pub fn notify(text: &str, duration_ms: u64) {
    let mut buf = [0u8; 257];
    let bytes = text.as_bytes();
    let len = bytes.len().min(256);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
    unsafe { crate::syscall::syscall3(SYS_NOTIFY, buf.as_ptr() as u64, duration_ms, 0); }
}

pub fn notify_with_type(text: &str, duration_ms: u64, kind: u64) {
    let mut buf = [0u8; 257];
    let bytes = text.as_bytes();
    let len = bytes.len().min(256);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
    unsafe { crate::syscall::syscall3(SYS_NOTIFY, buf.as_ptr() as u64, duration_ms, kind); }
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
