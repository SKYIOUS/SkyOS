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
    let mut lines: alloc::vec::Vec<&str> = text.lines().collect();
    while let Some(line) = lines.pop() {
        let _ = io::write(1, line.as_bytes());
        let _ = io::write(1, b"\n");
    }
    0
}

sarga_main!(user_main);
