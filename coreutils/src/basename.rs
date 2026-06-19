#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use libsarga::{sarga_main, println, args};

fn user_main() -> i32 {
    if args::argc() < 2 { println!("Usage: basename <path> [suffix]"); return 0; }
    let path = args::get(1).unwrap_or("");
    let base = path.rsplit('/').filter(|s| !s.is_empty()).next().unwrap_or(path);
    let mut result = String::from(base);
    if args::argc() > 2 {
        if let Some(suffix) = args::get(2) {
            if result.ends_with(suffix) {
                let new_len = result.len() - suffix.len();
                result.truncate(new_len);
            }
        }
    }
    println!("{}", result);

    0
    0
}

sarga_main!(user_main);
