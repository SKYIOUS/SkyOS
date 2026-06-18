#![no_std]
#![no_main]
use libsarga::{sarga_main, syscall};

fn user_main() -> i32 {
    unsafe { syscall::syscall0(36); }
    0
    0
}

sarga_main!(user_main);
