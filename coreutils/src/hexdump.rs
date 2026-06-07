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

fn user_main() {
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
        write!(w, "{:08x} ", offset).ok();
        for i in 0..16 {
            if i < n { write!(w, "{:02x} ", buf[i]).ok(); }
            else { write!(w, "   ").ok(); }
            if i == 7 { write!(w, " ").ok(); }
        }
        write!(w, " |").ok();
        for i in 0..n {
            let c = buf[i];
            if c >= 0x20 && c <= 0x7e { write!(w, "{}", c as char).ok(); }
            else { write!(w, ".").ok(); }
        }
        write!(w, "|\n").ok();
        offset += n;
    }
    if fd != 0 { let _ = io::close(fd); }
}

sarga_main!(user_main);
