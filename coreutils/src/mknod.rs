#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, println, sarga_main};

fn user_main() -> i32 {
    if args::argc() < 4 {
        println!("usage: mknod name type major minor");
        return 1;
    }
    println!(
        "mknod: {}: not supported on this system",
        args::get(1).unwrap_or("")
    );
    1
}
sarga_main!(user_main);
