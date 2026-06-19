#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::{sarga_main, println, args, io, syscall};

fn join_path(base: &str, name: &str) -> String {
    if base == "/" {
        alloc::format!("/{}", name)
    } else {
        alloc::format!("{}/{}", base, name)
    }
}

fn remove_recursive(path: &str, force: bool) -> i64 {
    let mut path_c = String::from(path);
    path_c.push('\0');
    let fd = match io::open(&path_c, 0) {
        Ok(fd) => fd,
        Err(_) => { if !force { println!("rm: {}: not found", path); } return -1; }
    };
    let mut buf = [0u8; 4096];
    loop {
        let r = unsafe { syscall::syscall3(78, fd as u64, buf.as_mut_ptr() as u64, 4096) };
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
                    let entry_path = join_path(path, name);
                    match libsarga::fs::stat(&entry_path) {
                        Ok(st) => {
                            if (st.mode & 0o170000) == 0o040000 {
                                remove_recursive(&entry_path, force);
                                let mut ep_c = String::from(&entry_path);
                                ep_c.push('\0');
                                unsafe { syscall::syscall1(84, ep_c.as_ptr() as u64); }
                            } else {
                                let mut ep_c = String::from(&entry_path);
                                ep_c.push('\0');
                                unsafe { syscall::syscall1(87, ep_c.as_ptr() as u64); }
                            }
                        }
                        Err(_) => {}
                    }
                }
            }
            i += reclen;
        }
    }
    let _ = io::close(fd);
    0
}

fn user_main() -> i32 {
    if args::argc() < 2 {
        println!("Usage: rm [-rf] <path>...");
        return 1;
    }
    let mut recursive = false;
    let mut force = false;
    let mut paths = Vec::new();
    for i in 1..args::argc() {
        if let Some(s) = args::get(i as usize) {
            if s == "-r" || s == "-rf" || s == "-fr" { recursive = true; }
            else if s == "-f" { force = true; }
            else if s.starts_with('-') { continue; }
            else { paths.push(String::from(s)); }
        }
    }
    let mut exit_code = 0;
    for path in &paths {
        let mut path_c = String::from(path);
        path_c.push('\0');
        match libsarga::fs::stat(path) {
            Ok(st) => {
                let is_dir = (st.mode & 0o170000) == 0o040000;
                if is_dir && recursive {
                    remove_recursive(path, force);
                    unsafe { syscall::syscall1(84, path_c.as_ptr() as u64); }
                } else if is_dir && !recursive {
                    println!("rm: {}: is a directory (use -r)", path);
                    exit_code = 1;
                } else {
                    let r = unsafe { syscall::syscall1(87, path_c.as_ptr() as u64) };
                    if r != 0 && !force { println!("rm: {}: failed with error {}", path, -r); exit_code = 1; }
                }
            }
            Err(e) => {
                if !force { println!("rm: {}: stat failed with error {}", path, e); exit_code = 1; }
            }
        }
    }
    exit_code
    0
    0
}

sarga_main!(user_main);
