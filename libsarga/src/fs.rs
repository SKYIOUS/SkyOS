use crate::syscall::*;

#[repr(C)]
pub struct Stat {
    pub dev: u64, pub ino: u64, pub mode: u32, pub nlink: u32,
    pub uid: u32, pub gid: u32, pub rdev: u64, pub size: i64,
    pub blksize: i64, pub blocks: i64,
    pub atime: u64, pub mtime: u64, pub ctime: u64,
}

#[repr(C)]
pub struct StatFs {
    pub f_type: u64, pub f_bsize: u64,
    pub f_blocks: u64, pub f_bfree: u64, pub f_bavail: u64,
    pub f_files: u64, pub f_ffree: u64,
}

pub fn stat(path: &str) -> Result<Stat, i64> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let mut s = core::mem::MaybeUninit::<Stat>::uninit();
    let r = unsafe { syscall2(4, buf.as_ptr() as u64, s.as_mut_ptr() as u64) };
    if r < 0 { Err(-r) } else { Ok(unsafe { s.assume_init() }) }
}

pub fn fstat(fd: i64) -> Result<Stat, i64> {
    let mut s = core::mem::MaybeUninit::<Stat>::uninit();
    let r = unsafe { syscall2(5, fd as u64, s.as_mut_ptr() as u64) };
    if r < 0 { Err(-r) } else { Ok(unsafe { s.assume_init() }) }
}

pub fn statfs(path: &str) -> Result<StatFs, i64> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let mut s = core::mem::MaybeUninit::<StatFs>::uninit();
    let r = unsafe { syscall2(137, buf.as_ptr() as u64, s.as_mut_ptr() as u64) };
    if r < 0 { Err(-r) } else { Ok(unsafe { s.assume_init() }) }
}

pub fn touch(path: &str) -> i64 {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return -crate::errno::EINVAL as i64; }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let fd = unsafe { syscall2(2, buf.as_ptr() as u64, 0x241u64) };
    if fd >= 0 { unsafe { syscall1(3, fd as u64) }; }
    0
}

pub fn open(path: &str, flags: u64) -> Result<i64, i64> {
    let mut buf = [0u8; 256];
    let bytes = path.as_bytes();
    if bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let r = unsafe { syscall2(2, buf.as_ptr() as u64, flags) };
    if r < 0 { Err(-r) } else { Ok(r) }
}

pub fn read(fd: i64, buf: &mut [u8]) -> Result<usize, i64> {
    let r = unsafe { syscall3(0, fd as u64, buf.as_mut_ptr() as u64, buf.len() as u64) };
    if r < 0 { Err(-r) } else { Ok(r as usize) }
}

pub fn write(fd: i64, buf: &[u8]) -> Result<usize, i64> {
    let r = unsafe { syscall3(1, fd as u64, buf.as_ptr() as u64, buf.len() as u64) };
    if r < 0 { Err(-r) } else { Ok(r as usize) }
}

pub fn close(fd: i64) -> i64 {
    unsafe { syscall1(3, fd as u64) }
}

pub fn mkfs(fstype: &str, device: u64) -> Result<(), i64> {
    let mut fs_buf = [0u8; 32];
    let fs_bytes = fstype.as_bytes();
    if fs_bytes.len() > 30 { return Err(crate::errno::EINVAL as i64); }
    fs_buf[..fs_bytes.len()].copy_from_slice(fs_bytes);
    fs_buf[fs_bytes.len()] = 0;
    let r = unsafe { crate::syscall::syscall2(127, fs_buf.as_ptr() as u64, device) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn mount(source: &str, target: &str, fstype: &str, flags: u64) -> Result<(), i64> {
    let mut src_buf = [0u8; 256];
    let src_bytes = source.as_bytes();
    if src_bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    src_buf[..src_bytes.len()].copy_from_slice(src_bytes);
    src_buf[src_bytes.len()] = 0;

    let mut tgt_buf = [0u8; 256];
    let tgt_bytes = target.as_bytes();
    if tgt_bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    tgt_buf[..tgt_bytes.len()].copy_from_slice(tgt_bytes);
    tgt_buf[tgt_bytes.len()] = 0;

    let mut fs_buf = [0u8; 32];
    let fs_bytes = fstype.as_bytes();
    if fs_bytes.len() > 30 { return Err(crate::errno::EINVAL as i64); }
    fs_buf[..fs_bytes.len()].copy_from_slice(fs_bytes);
    fs_buf[fs_bytes.len()] = 0;

    let r = unsafe { syscall5(165, src_buf.as_ptr() as u64, tgt_buf.as_ptr() as u64, fs_buf.as_ptr() as u64, flags, 0) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn umount(target: &str) -> Result<(), i64> {
    let mut buf = [0u8; 256];
    let bytes = target.as_bytes();
    if bytes.len() > 254 { return Err(crate::errno::EINVAL as i64); }
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    let r = unsafe { syscall2(167, buf.as_ptr() as u64, 0) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn read_to_string(path: &str) -> Result<alloc::string::String, i64> {
    let fd = open(path, 0)?;
    let mut data = alloc::vec::Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        match read(fd, &mut tmp) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&tmp[..n]),
            Err(e) => { close(fd); return Err(e); }
        }
    }
    close(fd);
    Ok(alloc::string::String::from_utf8_lossy(&data).into_owned())
}

pub fn write_file(path: &str, content: &str) -> Result<(), i64> {
    let fd = open(path, 0x241 | 0x200)?;
    let mut written = 0;
    let bytes = content.as_bytes();
    while written < bytes.len() {
        match write(fd, &bytes[written..]) {
            Ok(n) => written += n,
            Err(e) => { close(fd); return Err(e); }
        }
    }
    close(fd);
    Ok(())
}
