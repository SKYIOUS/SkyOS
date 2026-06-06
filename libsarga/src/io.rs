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

pub fn print_str(s: &str) { let _ = write_all(1, s.as_bytes()); }

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => { $crate::io::print_str(&$crate::alloc::format!($($arg)*)) }
}
#[macro_export]
macro_rules! println {
    () => { $crate::print!("\n") };
    ($($arg:tt)*) => { $crate::io::print_str(&$crate::alloc::format!("{}\n", $($arg)*)) }
}
