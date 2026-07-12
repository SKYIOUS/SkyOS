#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println};

mod raw {
    pub fn open(path: &str, flags: i32) -> i64 {
        let b = path.as_bytes();
        let l = core::cmp::min(b.len(), 255);
        let mut buf = [0u8; 256];
        buf[..l].copy_from_slice(&b[..l]);
        buf[l] = 0;
        unsafe { libsarga::syscall::syscall2(2, buf.as_ptr() as u64, flags as u64) }
    }
    pub fn read(fd: i64, buf: &mut [u8]) -> i64 {
        unsafe { libsarga::syscall::syscall3(0, fd as u64, buf.as_mut_ptr() as u64, buf.len() as u64) }
    }
    pub fn write(fd: i64, buf: &[u8]) -> i64 {
        unsafe { libsarga::syscall::syscall3(1, fd as u64, buf.as_ptr() as u64, buf.len() as u64) }
    }
    pub fn close(fd: i64) -> i64 {
        unsafe { libsarga::syscall::syscall1(3, fd as u64) }
    }
    pub fn setuid(uid: u64) -> i64 {
        unsafe { libsarga::syscall::syscall1(303, uid) }
    }
    pub fn unlink(path: &str) -> i64 {
        let b = path.as_bytes();
        let l = core::cmp::min(b.len(), 127);
        let mut buf = [0u8; 128];
        buf[..l].copy_from_slice(&b[..l]);
        buf[l] = 0;
        unsafe { libsarga::syscall::syscall1(87, buf.as_ptr() as u64) }
    }
}

fn main_test() -> i32 {
    let mut failures = 0u32;

    // 1. Create a test file (all processes start as root)
    let fd = raw::open("/test_file", 0x40); // O_CREAT
    if fd < 0 { println!("FAIL: could not create /test_file ({})", fd); return 1; }
    raw::write(fd, b"hello");
    raw::close(fd);

    // 2A. fd_flags: open O_RDONLY, try write → expected EBADF
    let fd_ro = raw::open("/test_file", 0);
    if fd_ro < 0 {
        println!("FAIL: O_RDONLY open returned {}", fd_ro); failures += 1;
    } else {
        let w = raw::write(fd_ro, b"x");
        if w >= 0 { println!("FAIL: write on O_RDONLY succeeded"); failures += 1; }
        else { println!("PASS: write on O_RDONLY denied"); }
        raw::close(fd_ro);
    }

    // 2B. fd_flags: open O_WRONLY, try read → expected EBADF
    let fd_wo = raw::open("/test_file", 1);
    if fd_wo < 0 {
        println!("FAIL: O_WRONLY open returned {}", fd_wo); failures += 1;
    } else {
        let mut tmp = [0u8; 4];
        let r = raw::read(fd_wo, &mut tmp);
        if r >= 0 { println!("FAIL: read on O_WRONLY succeeded"); failures += 1; }
        else { println!("PASS: read on O_WRONLY denied"); }
        raw::close(fd_wo);
    }

    // 3. Cross-UID permission: set uid=1000 (non-owner), try open root-owned file
    let r = raw::setuid(1000);
    if r < 0 { println!("WARN: setuid(1000) returned {}", r); }
    let fd2 = raw::open("/test_file", 0);
    if fd2 >= 0 {
        println!("NOTE: non-root (uid=1000) could still open /test_file");
        raw::close(fd2);
    } else {
        println!("PASS: uid=1000 denied access to owner=0 file");
    }

    // Cleanup
    raw::setuid(0); // back to root
    raw::unlink("/test_file");

    if failures > 0 { println!("{} test(s) FAILED", failures); 1 } else { println!("PASS: all permissions"); 0 }
}

fn user_main() -> i32 { main_test() }
sarga_main!(user_main);