#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, io};

fn user_main() -> i32 {
    let mut lines = 0u64;
    let mut words = 0u64;
    let mut bytes = 0u64;
    let mut buf = [0u8; 1024];

    loop {
        let n = match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        bytes += n as u64;
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        for c in s.chars() {
            if c == '\n' { lines += 1; }
        }
        let mut in_word = false;
        for c in s.chars() {
            if c == ' ' || c == '\n' || c == '\t' || c == '\r' {
                in_word = false;
            } else if !in_word {
                words += 1;
                in_word = true;
            }
        }
    }
    println!("{} {} {} {}", lines, words, bytes, args::get(1).unwrap_or(""));

    0
}

sarga_main!(user_main);
