#![no_std]
#![no_main]
use libsarga::{sarga_main, println, args};
use libsarga::fs;

fn user_main() -> i32 {
    if args::argc() < 2 {
        println!("Usage: umount <target>");
        return 0;
    }
    let target = match args::get(1) {
        Some(s) => s,
        None => { println!("umount: missing operand"); return 0; }
    };
    if let Err(e) = fs::umount(target) {
        println!("umount: {}: error {}", target, e);
        return 1;
    }
    0
}
sarga_main!(user_main);
