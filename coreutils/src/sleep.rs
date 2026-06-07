#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, syscall::*};

fn user_main() {
    if args::argc() < 2 {
        println!("Usage: sleep <seconds>");
        libsarga::process::exit(1);
    }
    let secs: u64 = args::get(1).unwrap_or("0").parse().unwrap_or(0);
    let ns_spec = if secs > 0 { secs * 1_000_000_000 } else { 100_000_000 };
    let _ = unsafe { syscall1(35, ns_spec) };
}

sarga_main!(user_main);
