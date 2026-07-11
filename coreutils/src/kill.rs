#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, syscall::*};

fn user_main() -> i32 {
    if args::argc() < 2 {
        println!("Usage: kill [-signal] <pid>");
        return 1;
    }
    let mut sig = 15u64;
    let mut start = 1;
    if let Some(s) = args::get(1) {
        if s.starts_with('-') {
            sig = s[1..].parse().unwrap_or(15);
            start = 2;
        }
    }
    for i in start..args::argc() {
        if let Some(pid_str) = args::get(i as usize) {
            let pid: i64 = pid_str.parse().unwrap_or(0);
            if pid > 0 {
                let _ = unsafe { syscall2(62, pid as u64, sig) };
            }
        }
    }
    0
}

sarga_main!(user_main);
