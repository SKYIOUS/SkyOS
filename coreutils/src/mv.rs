#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, io, syscall};

fn user_main() {
    if args::argc() < 3 {
        println!("Usage: mv <source> <dest>");
        libsarga::process::exit(1);
    }
    let src = args::get(1).unwrap_or("");
    let dst = args::get(2).unwrap_or("");
    if src.is_empty() || dst.is_empty() { libsarga::process::exit(1); }

    let r = unsafe { syscall::syscall2(82, src.as_ptr() as u64, dst.as_ptr() as u64) };
    if r != 0 {
        let src_fd = match io::open(src, 0) {
            Ok(fd) => fd,
            Err(_) => { println!("mv: {}: not found", src); libsarga::process::exit(1); }
        };
        let dst_fd = unsafe { syscall::syscall2(2, dst.as_ptr() as u64, 0o100 | 0x42) };
        if (dst_fd as i64) < 0 {
            println!("mv: {} -> {}: failed", src, dst);
            libsarga::process::exit(1);
        }
        let mut buf = [0u8; 65536];
        loop {
            let n = unsafe { syscall::syscall3(0, src_fd as u64, buf.as_mut_ptr() as u64, 65536) };
            if (n as i64) <= 0 { break; }
            unsafe { syscall::syscall3(1, dst_fd as u64, buf.as_ptr() as u64, n as u64); }
        }
        unsafe { syscall::syscall1(3, src_fd as u64); }
        unsafe { syscall::syscall1(3, dst_fd as u64); }
        unsafe { syscall::syscall1(87, src.as_ptr() as u64); }
    }
}

sarga_main!(user_main);
