#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, println, sarga_main};

fn user_main() -> i32 {
    if args::argc() < 2 {
        println!("usage: mkfifo name");
        return 1;
    }
    println!(
        "mkfifo: {}: not supported on this system",
        args::get(1).unwrap_or("")
    );
    1
}
sarga_main!(user_main);
