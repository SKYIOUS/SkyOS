#![no_std]
#![no_main]
use libsarga::sarga_main;

fn user_main() {}

sarga_main!(user_main);
