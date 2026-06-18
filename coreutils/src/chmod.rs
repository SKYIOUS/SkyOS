#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use libsarga::io::{self, open, close, fchmod};
use libsarga::sarga_main;

fn parse_mode(s: &str) -> Option<u32> {
    if s.as_bytes().iter().all(|&b| b >= b'0' && b <= b'7') {
        return u32::from_str_radix(s, 8).ok();
    }
    None
}

fn user_main() -> i32 {
    let argc = libsarga::args::argc();

    if argc < 3 {
        io::print_str("Usage: chmod <mode> <file>\n");
        return 0;
    }

    let mode_str = libsarga::args::get(1).unwrap_or("644");
    let mode = match parse_mode(mode_str) {
        Some(m) => m,
        None => {
            io::print_str(&alloc::format!("chmod: invalid mode: {}\n", mode_str));
            return 1;
        }
    };

    for i in 2..argc as usize {
        let file = match libsarga::args::get(i) {
            Some(f) => f,
            None => continue,
        };
        let path = alloc::format!("{}\0", file);
        let fd = match open(&path, 0) {
            Ok(f) => f,
            Err(e) => {
                io::print_str(&alloc::format!("chmod: {}: {}\n", file, e));
                continue;
            }
        };
        let ret = fchmod(fd as u64, mode);
        close(fd).ok();
        if ret < 0 {
            io::print_str(&alloc::format!("chmod: {}: failed\n", file));
        }
    }
    return 0;
    0
}

sarga_main!(user_main);
