#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, syscall::*};

fn user_main() {
    println!("Sarga OS 0.1.0-alpha");
}

sarga_main!(user_main);
