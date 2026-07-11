#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, io, args};
use core::fmt::Write;

struct StdoutWriter;
impl core::fmt::Write for StdoutWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let _ = io::write_all(1, s.as_bytes());
        Ok(())
    }
}

fn user_main() -> i32 {
    let fd = if args::argc() > 1 {
        let path = args::get(1).unwrap_or_default();
        io::open(&path, 0).unwrap_or(0)
    } else { 0 };
    let mut buf = [0u8; 16];
    let mut offset = 0usize;
    loop {
        let n = match io::read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        let mut w = StdoutWriter;
        write!(w, "{:07o} ", offset).ok();
        for i in 0..n {
            write!(w, "{:03o}", buf[i]).ok();
            if i % 2 == 1 { write!(w, " ").ok(); }
        }
        write!(w, "\n").ok();
        offset += n;
    }
    if fd != 0 { let _ = io::close(fd); }
    0
}

sarga_main!(user_main);
