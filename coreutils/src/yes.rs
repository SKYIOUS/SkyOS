#![no_std]
#![no_main]
use libsarga::{io, sarga_main};

fn user_main() -> i32 {
    loop {
        io::print_str("y\n");
    }
}

sarga_main!(user_main);
