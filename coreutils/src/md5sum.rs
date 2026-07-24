#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use core::fmt::Write;
use libsarga::{args, io, print, println, sarga_main};

fn md5(data: &[u8]) -> [u8; 16] {
    // Simple non-cryptographic hash for now (ponytail: real MD5 later)
    let mut h = [0u32; 4];
    for (i, &b) in data.iter().enumerate() {
        h[i % 4] = h[i % 4].wrapping_add(b as u32);
        h[i % 4] = h[i % 4].wrapping_mul(2654435761);
    }
    let mut out = [0u8; 16];
    for i in 0..4 { out[i*4..][..4].copy_from_slice(&h[i].to_le_bytes()); }
    out
}

fn user_main() -> i32 {
    if args::argc() < 2 {
        let mut buf = [0u8; 4096]; let mut all = alloc::vec::Vec::new();
        loop { match io::read(0, &mut buf) { Ok(0) => break, Ok(n) => all.extend_from_slice(&buf[..n]), Err(_) => break, } }
        let hash = md5(&all);
        let mut s = String::new();
        for &b in &hash { let _ = write!(s, "{:02x}", b); }
        println!("{}  -", s);
        return 0;
    }
    for i in 1..args::argc() as usize {
        let path = args::get(i).unwrap_or("");
        match io::read_to_string(path) {
            Ok(content) => {
                let hash = md5(content.as_bytes());
                let mut s = String::new();
                for &b in &hash { let _ = write!(s, "{:02x}", b); }
                println!("{}  {}", s, path);
            }
            Err(_) => { println!("md5sum: {}: No such file", path); return 1; }
        }
    }
    0
}
sarga_main!(user_main);