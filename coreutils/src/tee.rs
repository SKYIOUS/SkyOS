#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::{sarga_main, println, args, io, syscall};

fn user_main() {
    let mut append = false;
    let mut files = Vec::new();
    for i in 1..args::argc() {
        if let Some(s) = args::get(i as usize) {
            if s == "-a" { append = true; }
            else if s.starts_with('-') { continue; }
            else { files.push(String::from(s)); }
        }
    }
    let mut fds = Vec::new();
    for f in &files {
        let flags = if append { 0x401 } else { 0x100 | 0x42 };
        let fd = unsafe { syscall::syscall2(2, f.as_ptr() as u64, flags) };
        if (fd as i64) < 0 { println!("tee: {}: open failed", f); }
        else { fds.push(fd); }
    }
    let mut buf = [0u8; 4096];
    loop {
        let n = match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        io::write_all(1, &buf[..n]).ok();
        for &fd in &fds {
            unsafe { syscall::syscall3(1, fd as u64, buf.as_ptr() as u64, n as u64); }
        }
    }
    for &fd in &fds { let _ = io::close(fd); }
}

sarga_main!(user_main);
