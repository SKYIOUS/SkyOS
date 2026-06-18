#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use libsarga::{sarga_main, println, io, args};

fn find_dir(dir: &str, name: &str, depth: u32) {
    if depth > 64 { return 0; }
    let mut name_bytes = alloc::vec![0u8; 4096];
    let fd = match io::open(dir, 0) {
        Ok(fd) => fd,
        Err(_) => return,
    };
    loop {
        let r = unsafe { libsarga::syscall::syscall3(78, fd as u64, name_bytes.as_mut_ptr() as u64, 4096) };
        if (r as i64) <= 0 { break; }
        let entries = &name_bytes[..r as usize];
        let mut i = 0;
        while i + 16 <= entries.len() {
            let ino = u64::from_ne_bytes([entries[i], entries[i+1], entries[i+2], entries[i+3], entries[i+4], entries[i+5], entries[i+6], entries[i+7]]);
            let reclen = u16::from_ne_bytes([entries[i+8], entries[i+9]]) as usize;
            let namelen = entries[i+10] as usize;
            if reclen < 11 || i + reclen > entries.len() { break; }
            if namelen > 0 && i + 11 + namelen <= entries.len() {
                if let Ok(entry_name) = core::str::from_utf8(&entries[i+11..i+11+namelen]) {
                    if entry_name != "." && entry_name != ".." {
                        let full = if dir == "/" { alloc::format!("/{}", entry_name) } else { alloc::format!("{}/{}", dir, entry_name) };
                        let matched = name.is_empty() || full.contains(name);
                        if matched { println!("{}", full); }
                        if ino != 0 {
                            let mut st = [0u64; 32];
                            let r2 = unsafe { libsarga::syscall::syscall2(4, full.as_ptr() as u64, st.as_mut_ptr() as u64) };
                            if r2 == 0 && (st[1] & 0o170000) == 0o040000 {
                                find_dir(&full, name, depth + 1);
                            }
                        }
                    }
                }
            }
            i += reclen;
        }
    }
    let _ = io::close(fd);
}

fn user_main() -> i32 {
    let start_dir = if args::argc() > 1 { args::get(1).unwrap_or(".") } else { "." };
    let filter = if args::argc() > 2 { args::get(2).unwrap_or("") } else { "" };
    let sd = String::from(start_dir);
    let fl = String::from(filter);
    find_dir(&sd, &fl, 0);

    0
    0
}

sarga_main!(user_main);
