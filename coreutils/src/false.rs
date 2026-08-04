#![no_std]
#![no_main]
use libsarga::sarga_main;

fn user_main() -> i32 {
    1
}

sarga_main!(user_main);
