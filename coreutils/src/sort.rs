#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use libsarga::{sarga_main, println, io, args};

fn user_main() -> i32 {
    let mut lines = Vec::new();
    let mut buf = [0u8; 1024];
    let fd = if args::argc() > 1 {
        let path = args::get(1).unwrap_or_default();
        io::open(&path, 0).unwrap_or(0)
    } else { 0 };
    let mut current = String::new();
    loop {
        let n = match io::read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        for &b in &buf[..n] {
            if b == b'\n' {
                lines.push(current.clone());
                current.clear();
            } else { current.push(b as char); }
        }
    }
    if !current.is_empty() { lines.push(current); }
    if fd != 0 { let _ = io::close(fd); }
    lines.sort();
    for line in &lines { println!("{}", line); }
    0
    0
}

sarga_main!(user_main);
