#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, syscall::*};

fn user_main() {
    if args::argc() < 3 {
        println!("Usage: mv <source> <dest>");
        libsarga::process::exit(1);
    }
    let src = args::get(1).unwrap_or("");
    let dst = args::get(2).unwrap_or("");
    if src.is_empty() || dst.is_empty() { libsarga::process::exit(1); }

    let r = unsafe { syscall2(82, src.as_ptr() as u64, dst.as_ptr() as u64) };
    if r != 0 { println!("mv: {} -> {}: failed", src, dst); libsarga::process::exit(1); }
}

sarga_main!(user_main);
