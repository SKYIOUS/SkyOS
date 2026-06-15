#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use libsarga::{sarga_main, println, args};

fn user_main() {
    if args::argc() < 2 { println!("Usage: dirname <path>"); return; }
    let path = args::get(1).unwrap_or("");
    let dir = match path.rfind('/') {
        Some(0) => String::from("/"),
        Some(pos) => String::from(&path[..pos]),
        None => String::from("."),
    };
    println!("{}", dir);
}

sarga_main!(user_main);
