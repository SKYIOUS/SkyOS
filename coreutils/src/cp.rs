#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, syscall::*};
use alloc::vec::Vec;

fn user_main() {
    if args::argc() < 3 { println!("Usage: cp <src> <dst>"); return; }
    let src = args::get(1).unwrap_or("");
    let dst = args::get(2).unwrap_or("");
    let src_fd = unsafe { syscall2(2, src.as_ptr() as u64, 0) };
    if (src_fd as i64) < 0 { println!("cp: {}: not found", src); return; }
    let dst_fd = unsafe { syscall2(2, dst.as_ptr() as u64, 0o100 | 0x42) };
    if (dst_fd as i64) < 0 { println!("cp: {}: create failed", dst); unsafe { syscall1(3, src_fd as u64); } return; }
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { syscall3(0, src_fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (n as i64) <= 0 { break; }
        unsafe { syscall3(1, dst_fd as u64, buf.as_ptr() as u64, n as u64); }
    }
    unsafe { syscall1(3, src_fd as u64); }
    unsafe { syscall1(3, dst_fd as u64); }
}
sarga_main!(user_main);
