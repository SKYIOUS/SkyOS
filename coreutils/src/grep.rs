#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, io};
use alloc::string::String;

fn user_main() {
    if args::argc() < 2 {
        println!("Usage: grep <pattern> [file...]");
        libsarga::process::exit(1);
    }
    let pattern = args::get(1).unwrap_or("");
    if pattern.is_empty() { libsarga::process::exit(1); }

    let mut buf = [0u8; 1024];
    let mut leftover = String::new();
    loop {
        let n = match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        leftover.push_str(s);
        while let Some(pos) = leftover.find('\n') {
            let line = &leftover[..pos];
            if line.contains(pattern) {
                io::write_all(1, line.as_bytes()).ok();
                io::write_all(1, b"\n").ok();
            }
            leftover = String::from(&leftover[pos + 1..]);
        }
    }
    if !leftover.is_empty() && leftover.contains(pattern) {
        io::write_all(1, leftover.as_bytes()).ok();
        io::write_all(1, b"\n").ok();
    }
}

sarga_main!(user_main);
