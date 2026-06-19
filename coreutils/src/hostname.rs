#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, io, syscall};

fn user_main() -> i32 {
    if args::argc() > 1 {
        let name = args::get(1).unwrap_or("");
        let fd = unsafe { syscall::syscall2(2, "/ctl/kernel/hostname\0".as_ptr() as u64, 0x42) };
        if (fd as i64) >= 0 {
            unsafe { syscall::syscall3(1, fd as u64, name.as_ptr() as u64, name.len() as u64); }
            let _ = io::close(fd);
        } else { println!("hostname: failed to set hostname"); }
    } else {
        let fd = match io::open("/ctl/kernel/hostname", 0) {
            Ok(fd) => fd,
            Err(_) => { println!("hostname: failed to read hostname"); return 0; }
        };
        let mut buf = [0u8; 256];
        if let Ok(n) = io::read(fd, &mut buf) {
            if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                println!("{}", s.trim());
            }
        }
        let _ = io::close(fd);
    }
    0
    0
}

sarga_main!(user_main);
