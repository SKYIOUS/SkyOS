#![no_std]
#![no_main]
use libsarga::{sarga_main, syscall};

fn user_main() {
    unsafe { syscall::syscall0(36); }
}

sarga_main!(user_main);
