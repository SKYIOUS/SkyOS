#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::{args, fs, io, print, println, sarga_main};

fn user_main() -> i32 {
    let fd = match io::open("/dev", 0) {
        Ok(fd) => fd,
        Err(_) => {
            println!("blkid: no block devices available");
            return 0;
        }
    };

    let mut buf = [0u8; 4096];
    let mut devices: Vec<(u64, u32, String)> = Vec::new();
    loop {
        let n = match io::getdents64(fd, &mut buf) {
            Ok(n) if n > 0 => n,
            _ => break,
        };
        let mut off = 0;
        while off < n {
            let ino = u64::from_ne_bytes(buf[off..off + 8].try_into().unwrap_or([0; 8]));
            let reclen =
                u16::from_ne_bytes(buf[off + 16..off + 18].try_into().unwrap_or([0; 2])) as usize;
            let name_start = off + 19;
            let name_end = buf[name_start..].iter().position(|&c| c == 0).unwrap_or(0);
            if ino != 0 && name_end > 0 {
                if let Ok(name) = core::str::from_utf8(&buf[name_start..name_start + name_end]) {
                    if name != "." && name != ".." {
                        let mode = fs::stat(&alloc::format!("/dev/{}", name))
                            .map(|s| s.mode)
                            .unwrap_or(0);
                        devices.push((ino, mode, String::from(name)));
                    }
                }
            }
            if reclen == 0 {
                break;
            }
            off += reclen;
        }
    }
    let _ = io::close(fd);

    if devices.is_empty() {
        println!("blkid: no block devices available");
        return 0;
    }

    for (ino, mode, name) in &devices {
        let ftype = if mode & 0o170000 == 0o060000 {
            "block"
        } else if mode & 0o170000 == 0o020000 {
            "char"
        } else if mode & 0o170000 == 0o040000 {
            "dir"
        } else {
            "unknown"
        };
        println!("{} {:>8}  {}", ftype, ino, name);
    }
    0
}

sarga_main!(user_main);
