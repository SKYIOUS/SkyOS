#![no_std]
#![no_main]
use libsarga::{io, sarga_main};

fn user_main() -> i32 {
    io::print_str("\x1b[2J\x1b[H");
    0
}

sarga_main!(user_main);
