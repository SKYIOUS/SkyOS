#![no_std]
#![no_main]
use libsarga::{sarga_main, io};

fn user_main() -> i32 {
    loop {
        io::print_str("y\n");
    }
}

sarga_main!(user_main);
