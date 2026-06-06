#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use libsarga::{sarga_main, print, println, args, io};

fn join_path(base: &str, name: &str) -> String {
    if base == "/" {
        alloc::format!("/{}", name)
    } else if base.ends_with('/') {
        alloc::format!("{}{}", base, name)
    } else if base == "." {
        String::from(name)
    } else {
        alloc::format!("{}/{}", base, name)
    }
}

fn user_main() {
    let path = if args::argc() > 1 { args::get(1).unwrap_or(".") } else { "." };
    let fd = match io::open(path, 0) {
        Ok(fd) => fd,
        Err(_) => { println!("ls: {}: not found", path); return; }
    };
    let mut buf = [0u8; 4096];
    loop {
        let r = unsafe { libsarga::syscall::syscall3(78, fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (r as i64) <= 0 { break; }
        let entries = &buf[..r as usize];
        let mut i = 0;
        while i + 16 <= entries.len() {
            let reclen = u16::from_ne_bytes([entries[i+8], entries[i+9]]) as usize;
            let namelen = entries[i+10] as usize;
            if reclen < 11 || i + reclen > entries.len() { break; }
            if namelen > 0 && i + 11 + namelen <= entries.len() {
                if let Ok(name) = core::str::from_utf8(&entries[i+11..i+11+namelen]) {
                    if name == "." || name == ".." { continue; }
                    let entry_path = join_path(path, name);
                    let mut st = [0u64; 32];
                    let r2 = unsafe { libsarga::syscall::syscall2(4, entry_path.as_ptr() as u64, st.as_mut_ptr() as u64) };
                    if r2 == 0 && (st[1] & 0o170000) == 0o040000 {
                        print!("d ");
                    } else if r2 == 0 {
                        print!("- ");
                    } else {
                        print!("? ");
                    }
                    println!("{}", name);
                }
            }
            i += reclen;
        }
    }
    let _ = io::close(fd);
}

sarga_main!(user_main);
