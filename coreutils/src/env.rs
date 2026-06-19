#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println};

fn user_main() -> i32 {
    println!("PATH=/bin:/usr/bin");
    println!("HOME=/home/user");
    println!("SHELL=/bin/sash");

    0
    0
}

sarga_main!(user_main);
