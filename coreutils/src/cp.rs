#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::{sarga_main, println, args, io, syscall};

fn join_path(base: &str, name: &str) -> String {
    if base.ends_with('/') || base.is_empty() {
        alloc::format!("{}{}", base, name)
    } else {
        alloc::format!("{}/{}", base, name)
    }
}

fn copy_file(src: &str, dst: &str) -> i64 {
    let src_fd = match io::open(src, 0) {
        Ok(fd) => fd,
        Err(_) => { println!("cp: {}: not found", src); return -1; }
    };
    let dst_fd = unsafe { syscall::syscall2(2, dst.as_ptr() as u64, 0o100 | 0x42) };
    if (dst_fd as i64) < 0 {
        println!("cp: {}: create failed", dst);
        unsafe { syscall::syscall1(3, src_fd as u64); }
        return -1;
    }
    let mut buf = [0u8; 65536];
    loop {
        let n = unsafe { syscall::syscall3(0, src_fd as u64, buf.as_mut_ptr() as u64, 65536) };
        if (n as i64) <= 0 { break; }
        unsafe { syscall::syscall3(1, dst_fd as u64, buf.as_ptr() as u64, n as u64); }
    }
    unsafe { syscall::syscall1(3, src_fd as u64); }
    unsafe { syscall::syscall1(3, dst_fd as u64); }
    0
}

fn copy_recursive(src: &str, dst: &str) -> i64 {
    let src_fd = match io::open(src, 0) {
        Ok(fd) => fd,
        Err(_) => { println!("cp: {}: not found", src); return -1; }
    };
    unsafe { syscall::syscall3(83, dst.as_ptr() as u64, 0o755, 0); }
    let mut buf = [0u8; 4096];
    loop {
        let r = unsafe { syscall::syscall3(78, src_fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (r as i64) <= 0 { break; }
        let entries = &buf[..r as usize];
        let mut i = 0;
        while i + 16 <= entries.len() {
            let reclen = u16::from_ne_bytes([entries[i+8], entries[i+9]]) as usize;
            let namelen = entries[i+10] as usize;
            if reclen < 11 || i + reclen > entries.len() { break; }
            if namelen > 0 && i + 11 + namelen <= entries.len() {
                if let Ok(name) = core::str::from_utf8(&entries[i+11..i+11+namelen]) {
                    if name == "." || name == ".." { i += reclen; continue; }
                    let src_path = join_path(src, name);
                    let dst_path = join_path(dst, name);
                    let mut st = [0u64; 32];
                    if unsafe { syscall::syscall2(4, src_path.as_ptr() as u64, st.as_mut_ptr() as u64) } == 0 {
                        if (st[1] & 0o170000) == 0o040000 {
                            copy_recursive(&src_path, &dst_path);
                        } else {
                            copy_file(&src_path, &dst_path);
                        }
                    }
                }
            }
            i += reclen;
        }
    }
    let _ = io::close(src_fd);
    0
}

fn user_main() -> i32 {
    if args::argc() < 3 { println!("Usage: cp [-r] <src>... <dst>"); return 0; }
    let mut recursive = false;
    let mut preserve = false;
    let mut files = Vec::new();
    for i in 1..args::argc() {
        if let Some(s) = args::get(i as usize) {
            if s == "-r" || s == "-R" { recursive = true; }
            else if s == "-p" { preserve = true; }
            else if s == "-rp" || s == "-pr" { recursive = true; preserve = true; }
            else if s.starts_with('-') { continue; }
            else { files.push(String::from(s)); }
        }
    }
    if files.len() < 2 { println!("cp: missing operand"); return 0; }
    let dst = files.pop().unwrap();
    for src in &files {
        let mut st = [0u64; 32];
        let is_dir = unsafe { syscall::syscall2(4, src.as_ptr() as u64, st.as_mut_ptr() as u64) } == 0
            && (st[1] & 0o170000) == 0o040000;
        if is_dir && recursive {
            let dst_dir = if files.len() > 1 || dst.ends_with('/') {
                let base = src.rsplit('/').next().unwrap_or(src);
                join_path(&dst, base)
            } else {
                dst.clone()
            };
            copy_recursive(src, &dst_dir);
        } else if is_dir {
            println!("cp: {}: omitting directory (use -r)", src);
        } else {
            if files.len() > 0 || dst.ends_with('/') {
                let base = src.rsplit('/').next().unwrap_or(src);
                let dst_path = join_path(&dst, base);
                copy_file(src, &dst_path);
            } else {
                copy_file(src, &dst);
            }
        }
        if preserve {
            let mut st = [0u64; 32];
            unsafe { syscall::syscall2(4, src.as_ptr() as u64, st.as_mut_ptr() as u64); }
        }
    }
    0
    0
}

sarga_main!(user_main);
