#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, syscall::*};

fn user_main() {
    let mut info = [0u64; 16];
    let r = unsafe { syscall1(203, info.as_mut_ptr() as u64) };
    if r == 0 {
        let uptime = info[2];
        println!("Uptime: {}s", uptime);
    }
    println!("PID  COMMAND");
    println!("1    init");
    println!("--   sash");
}

sarga_main!(user_main);
