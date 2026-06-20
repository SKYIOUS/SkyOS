#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::io;
use libsarga::args;

fn user_main() -> i32 {
    let mut count = false;
    let mut i = 1;
    while i < args::argc() {
        let arg = args::get(i as usize).unwrap_or("");
        if arg == "-c" { count = true; }
        i += 1;
    }
    let mut prev = alloc::string::String::new();
    let mut dup_count: u64 = 0;
    let mut buf = alloc::vec::Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        match io::read(0, &mut tmp) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
    }
    let text = alloc::string::String::from_utf8_lossy(&buf);
    for line in text.lines() {
        if line == prev {
            dup_count += 1;
        } else {
            if !prev.is_empty() {
                if count {
                    io::print_str(&alloc::format!("{} {}\n", dup_count, prev));
                } else {
                    io::print_str(&alloc::format!("{}\n", prev));
                }
            }
            prev.clear();
            prev.push_str(line);
            dup_count = 1;
        }
    }
    if !prev.is_empty() {
        if count {
            io::print_str(&alloc::format!("{} {}\n", dup_count, prev));
        } else {
            io::print_str(&alloc::format!("{}\n", prev));
        }
    }
    0
}

sarga_main!(user_main);
