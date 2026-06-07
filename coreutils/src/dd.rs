#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use libsarga::{sarga_main, println, io, args};

fn user_main() {
    let mut ifile = String::new();
    let mut ofile = String::new();
    let mut bs = 512usize;
    let mut count = usize::MAX;
    for i in 1..args::argc() {
        if let Some(arg) = args::get(i as usize) {
            if let Some(val) = arg.strip_prefix("if=") { ifile = String::from(val); }
            else if let Some(val) = arg.strip_prefix("of=") { ofile = String::from(val); }
            else if let Some(val) = arg.strip_prefix("bs=") { bs = val.parse().unwrap_or(512); }
            else if let Some(val) = arg.strip_prefix("count=") { count = val.parse().unwrap_or(usize::MAX); }
        }
    }
    let infd = if ifile.is_empty() { 0 } else { io::open(&ifile, 0).unwrap_or(0) };
    let outfd = if ofile.is_empty() { 1 } else { io::open(&ofile, 1).unwrap_or(1) };
    let mut buf = alloc::vec![0u8; bs];
    let mut total = 0usize;
    for _ in 0..count {
        let n = match io::read(infd, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        let mut written = 0;
        while written < n {
            let w = io::write(outfd, &buf[written..n]).unwrap_or(0);
            if w == 0 { break; }
            written += w;
        }
        total += n;
    }
    if ifile.is_empty() && infd != 0 { let _ = io::close(infd); }
    if !ofile.is_empty() { let _ = io::close(outfd); }
    println!("{} bytes copied", total);
}

sarga_main!(user_main);
