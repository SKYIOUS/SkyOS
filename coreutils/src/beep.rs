#![no_std]
#![no_main]
use libsarga::{sarga_main, io};

fn user_main() -> i32 {
    io::print_str("\x07");
    0
}

sarga_main!(user_main);
