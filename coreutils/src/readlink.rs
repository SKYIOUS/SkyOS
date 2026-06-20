#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, syscall};

fn user_main() -> i32 {
    if args::argc() < 2 { println!("Usage: readlink [-f] <path>"); return 1; }
    let path = args::get(1).unwrap_or("");
    if path.is_empty() { return 1; }
    let mut buf = [0u8; 1024];
    let r = unsafe { syscall::syscall3(89, path.as_ptr() as u64, buf.as_mut_ptr() as u64, 1023) };
    if (r as i64) < 0 { println!("readlink: {}: failed", path); return 1; }
    let len = r as usize;
    if let Ok(s) = core::str::from_utf8(&buf[..len]) {
        println!("{}", s);
    }
    0
}

sarga_main!(user_main);
