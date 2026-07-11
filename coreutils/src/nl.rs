#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::io;

fn user_main() -> i32 {
    let mut data = alloc::vec::Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    let text = alloc::string::String::from_utf8_lossy(&data);
    let mut line_num: u64 = 1;
    for line in text.lines() {
        io::print_str(&alloc::format!("{:>6}\t{}\n", line_num, line));
        line_num += 1;
    }
    0
}

sarga_main!(user_main);
