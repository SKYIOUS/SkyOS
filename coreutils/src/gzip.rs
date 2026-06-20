#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::io;

fn user_main() -> i32 {
    io::print_str("gzip: compression not supported in this build\n");
    io::print_str("(binary data passthrough)\n");
    let mut buf = [0u8; 4096];
    loop {
        match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => { let _ = io::write(1, &buf[..n]); }
            Err(_) => break,
        }
    }
    0
}

sarga_main!(user_main);
