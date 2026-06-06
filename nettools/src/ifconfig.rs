#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, print, println};

fn user_main() -> i32 {
    println!("ifconfig");
    0
}

sarga_main!(user_main);
