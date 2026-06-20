#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io, args};

fn user_main() -> i32 {
    let start = if args::argc() > 1 { 1 } else { 0 };
    if start == 0 {
        let mut buf = [0u8; 1024];
        loop {
            match io::read(0, &mut buf) {
                Ok(0) => break,
                Ok(n) => { let _ = io::write(1, &buf[..n]); }
                Err(e) => { println!("cat error: {}", e); return 1; }
            }
        }
        return 0;
    }
    let mut exit_code = 0;
    for i in start..args::argc() {
        if let Some(path) = args::get(i as usize) {
            let mut path_c = alloc::string::String::from(path);
            path_c.push('\0');
            let fd = unsafe { libsarga::syscall::syscall2(2, path_c.as_ptr() as u64, 0) };
            if (fd as i64) < 0 { println!("cat: {}: not found", path); exit_code = 1; continue; }
            let mut buf = [0u8; 1024];
            loop {
                let n = unsafe { libsarga::syscall::syscall3(0, fd as u64, buf.as_mut_ptr() as u64, 1024) };
                if (n as i64) <= 0 { break; }
                let _ = io::write(1, &buf[..n as usize]);
            }
            let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
        }
    }
    exit_code
}
sarga_main!(user_main);
