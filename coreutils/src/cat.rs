#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io, args};

fn user_main() {
    let start = if args::argc() > 1 { 1 } else { 0 };
    if start == 0 {
        let mut buf = [0u8; 1024];
        loop {
            match io::read(0, &mut buf) {
                Ok(0) => break,
                Ok(n) => { let _ = io::write(1, &buf[..n]); }
                Err(e) => { println!("cat error: {}", e); libsarga::process::exit(1); }
            }
        }
        return;
    }
    for i in start..args::argc() {
        if let Some(path) = args::get(i as usize) {
            let fd = unsafe { libsarga::syscall::syscall2(2, path.as_ptr() as u64, 0) };
            if (fd as i64) < 0 { println!("cat: {}: not found", path); continue; }
            let mut buf = [0u8; 1024];
            loop {
                let n = unsafe { libsarga::syscall::syscall3(0, fd as u64, buf.as_mut_ptr() as u64, 1024) };
                if (n as i64) <= 0 { break; }
                let _ = io::write(1, &buf[..n as usize]);
            }
            let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
        }
    }
}
sarga_main!(user_main);
