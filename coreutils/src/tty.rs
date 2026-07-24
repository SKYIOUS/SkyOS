#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, io, print, println, sarga_main};

fn user_main() -> i32 {
    let mut buf = [0u8; 256];
    let n = io::readlink("/proc/self/fd/0", &mut buf).unwrap_or(0);
    if n > 0 {
        let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
        if s.contains("pty") || s.contains("tty") || s.contains("console") {
            println!("{}", s); 0
        } else { println!("not a tty"); 1 }
    } else { println!("not a tty"); 1 }
}
sarga_main!(user_main);