#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println};

fn user_main() {
    println!("PATH=/bin:/usr/bin");
    println!("HOME=/home/user");
    println!("SHELL=/bin/sash");
}

sarga_main!(user_main);
