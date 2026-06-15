#![no_std]
#![no_main]
use libsarga::{sarga_main, println, args};
use libsarga::fs;

fn user_main() {
    if args::argc() < 2 {
        println!("Usage: umount <target>");
        return;
    }
    let target = match args::get(1) {
        Some(s) => s,
        None => { println!("umount: missing operand"); return; }
    };
    match fs::umount(target) {
        Ok(_) => {},
        Err(e) => println!("umount: {}: error {}", target, e),
    }
}
sarga_main!(user_main);
