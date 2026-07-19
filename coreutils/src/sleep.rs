#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, println, sarga_main, syscall::*};

fn user_main() -> i32 {
    if args::argc() < 2 {
        println!("Usage: sleep <seconds>");
        return 1;
    }
    let secs: u64 = args::get(1).unwrap_or("0").parse().unwrap_or(0);
    let _ = unsafe { syscall2(35, secs, 0) };

    0
}

sarga_main!(user_main);
