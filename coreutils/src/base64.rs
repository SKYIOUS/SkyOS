#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use libsarga::{args, io, println, sarga_main};

fn encode(data: &[u8]) -> String {
    STANDARD.encode(data)
}

fn decode(s: &str) -> alloc::vec::Vec<u8> {
    STANDARD.decode(s).unwrap_or_default()
}

fn user_main() -> i32 {
    let decode_mode = args::get(1) == Some("-d");
    let file_idx = if decode_mode { 2 } else { 1 };
    let data = if file_idx < args::argc() {
        let path = args::get(file_idx as usize).unwrap_or("");
        match io::read_to_string(path) {
            Ok(s) => s.into_bytes(),
            Err(_) => {
                println!("base64: {}: No such file", path);
                return 1;
            }
        }
    } else {
        let mut buf = [0u8; 4096];
        let mut all = alloc::vec::Vec::new();
        loop {
            match io::read(0, &mut buf) {
                Ok(0) => break,
                Ok(n) => all.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
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
