#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println};

fn user_main() -> i32 {
    println!("PATH=/bin:/usr/bin");
    println!("HOME=/home/root");
    println!("SHELL=/bin/sash");
    println!("USER=root");
    0
}

sarga_main!(user_main);
