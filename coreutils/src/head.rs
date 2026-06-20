#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, args, io};
use alloc::string::String;

fn user_main() -> i32 {
    let mut count: usize = 10;
    if args::argc() > 1 {
        if let Some(s) = args::get(1) {
            if s == "-n" {
                if let Some(n) = args::get(2) {
                    count = n.parse().unwrap_or(10);
                }
            }
        }
    }

    let mut buf = [0u8; 1024];
    let mut leftover = String::new();
    let mut printed = 0usize;
    loop {
        let n = match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        leftover.push_str(s);
        while let Some(pos) = leftover.find('\n') {
            if printed >= count { break; }
            io::write_all(1, leftover[..=pos].as_bytes()).ok();
            printed += 1;
            leftover = String::from(&leftover[pos + 1..]);
        }
    }
    if printed < count && !leftover.is_empty() {
        io::write_all(1, leftover.as_bytes()).ok();
    }
    0
}

sarga_main!(user_main);
