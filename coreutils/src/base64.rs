#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use libsarga::{args, io, print, println, sarga_main};

const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 { out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char); } else { out.push('='); }
        if chunk.len() > 2 { out.push(CHARS[(triple & 0x3F) as usize] as char); } else { out.push('='); }
    }
    out
}

fn decode(s: &str) -> alloc::vec::Vec<u8> {
    let mut out = alloc::vec::Vec::new();
    let mut buf = 0u32; let mut bits = 0;
    for &c in s.as_bytes() {
        let val = if c == b'=' { break }
            else if c == b'+' { 62 }
            else if c == b'/' { 63 }
            else if c >= b'A' && c <= b'Z' { (c - b'A') as u32 }
            else if c >= b'a' && c <= b'z' { (c - b'a' + 26) as u32 }
            else if c >= b'0' && c <= b'9' { (c - b'0' + 52) as u32 }
            else { continue };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 { bits -= 8; out.push((buf >> bits) as u8); buf &= (1 << bits) - 1; }
    }
    out
}

fn user_main() -> i32 {
    let decode_mode = args::get(1) == Some("-d");
    let file_idx = if decode_mode { 2 } else { 1 };
    let data = if file_idx < args::argc() {
        let path = args::get(file_idx as usize).unwrap_or("");
        match io::read_to_string(path) { Ok(s) => s.into_bytes(), Err(_) => { println!("base64: {}: No such file", path); return 1; } }
    } else {
        let mut buf = [0u8; 4096]; let mut all = alloc::vec::Vec::new();
        loop { match io::read(0, &mut buf) { Ok(0) => break, Ok(n) => all.extend_from_slice(&buf[..n]), Err(_) => break, } }
        all
    };

    if decode_mode {
        let decoded = decode(core::str::from_utf8(&data).unwrap_or(""));
        io::print_str(core::str::from_utf8(&decoded).unwrap_or(""));
    } else {
        let encoded = encode(&data);
        println!("{}", encoded);
    }
    0
}
sarga_main!(user_main);