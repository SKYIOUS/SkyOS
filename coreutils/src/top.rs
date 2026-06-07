#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, syscall::*};

fn user_main() {
    let mut info = [0u64; 32];
    let r = unsafe { syscall1(203, info.as_mut_ptr() as u64) };
    let uptime = if r == 0 { info[2] } else { 0 };
    println!("SkyOS - up {}s", uptime);
    println!("PID  PPID STATE COMMAND");
    println!("1    0    S     init");
    println!("2    1    S     sash");
    println!("--   --   R     top");
}

sarga_main!(user_main);
