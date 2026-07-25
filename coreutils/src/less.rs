#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::string::ToString;
use libsarga::{args, io, println, sarga_main};

fn user_main() -> i32 {
    let lines: alloc::vec::Vec<String> = if args::argc() > 1 {
        let path = args::get(1).unwrap();
        match io::read_to_string(path) {
            Ok(s) => s.lines().map(|l| l.to_string()).collect(),
            Err(_) => { println!("less: {}: No such file", path); return 1; }
        }
    } else {
        let mut buf = [0u8; 4096]; let mut all = String::new();
        loop { match io::read(0, &mut buf) { Ok(0) => break, Ok(n) => { if let Ok(s) = core::str::from_utf8(&buf[..n]) { all.push_str(s); } }, Err(_) => break, } }
        all.lines().map(|l| l.to_string()).collect()
    };

    let mut pos = 0usize;
    let rows = 24;
    loop {
        let end = core::cmp::min(pos + rows, lines.len());
        for j in pos..end { println!("{}", lines[j]); }
        pos = end;
        if pos >= lines.len() { break; }
        let mut k = [0u8; 1];
        let n = io::read(0, &mut k).unwrap_or(0);
        if n == 0 { break; }
        match k[0] {
            b'q' | b'Q' => break,
            b'u' | b'U' => pos = pos.saturating_sub(rows * 2),
            _ => {}
        }
    }
    0
}
sarga_main!(user_main);