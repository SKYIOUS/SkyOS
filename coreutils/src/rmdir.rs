#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, io, println, sarga_main};

fn user_main() -> i32 {
    if args::argc() < 2 {
        println!("Usage: rmdir <directory>");
        return 1;
    }
    let path = args::get(1).unwrap_or("");
    if path.is_empty() {
        return 1;
    }
    match io::unlink(path) {
        Ok(_) => 0,
        Err(e) => {
            println!("rmdir: failed to remove '{}': {:?}", path, e);
            1
        }
    }
}

sarga_main!(user_main);
