#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println};

fn user_main() -> i32 {
    println!("fdisk: no block devices available");
    0
}

sarga_main!(user_main);
