#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::{sarga_main, print, println, args, io, syscall};

fn join_path(base: &str, name: &str) -> String {
    if base == "/" {
        alloc::format!("/{}", name)
    } else if base.ends_with('/') {
        alloc::format!("{}{}", base, name)
    } else if base == "." || base == "" {
        String::from(name)
    } else {
        alloc::format!("{}/{}", base, name)
    }
}

fn human_size(size: u64) -> String {
    if size >= 1024 * 1024 * 1024 {
        alloc::format!("{:.1}G", size as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if size >= 1024 * 1024 {
        alloc::format!("{:.1}M", size as f64 / (1024.0 * 1024.0))
    } else if size >= 1024 {
        alloc::format!("{:.1}K", size as f64 / 1024.0)
    } else {
        alloc::format!("{}", size)
    }
}

fn user_main() -> i32 {
    let mut long = false;
    let mut all = false;
    let mut human = false;
    let mut path = ".";
    let mut arg_idx = 1;

    while arg_idx < args::argc() {
        let arg = args::get(arg_idx as usize).unwrap_or("");
        if arg == "-l" { long = true; arg_idx += 1; }
        else if arg == "-a" { all = true; arg_idx += 1; }
        else if arg == "-h" { human = true; arg_idx += 1; }
        else if arg == "-la" || arg == "-al" { long = true; all = true; arg_idx += 1; }
        else if arg == "-lh" || arg == "-hl" { long = true; human = true; arg_idx += 1; }
        else if arg == "-lah" || arg == "-lha" || arg == "-alh" { long = true; all = true; human = true; arg_idx += 1; }
        else if arg.starts_with('-') { arg_idx += 1; }
        else { path = arg; break; }
    }

    let mut path_c = String::from(path);
    path_c.push('\0');

    let fd = match io::open(&path_c, 0) {
        Ok(fd) => fd,
        Err(_) => { println!("ls: {}: not found", path); return 1; }
    };
    let mut buf = [0u8; 4096];
    loop {
        let r = unsafe { syscall::syscall3(78, fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (r as i64) <= 0 { break; }
        let entries = &buf[..r as usize];
        let mut i = 0;
        let mut names = Vec::new();
        while i + 16 <= entries.len() {
            let reclen = u16::from_ne_bytes([entries[i+8], entries[i+9]]) as usize;
            let namelen = entries[i+10] as usize;
            if reclen < 11 || i + reclen > entries.len() { break; }
            if namelen > 0 && i + 11 + namelen <= entries.len() {
                if let Ok(name) = core::str::from_utf8(&entries[i+11..i+11+namelen]) {
                    let is_dot = name == "." || name == "..";
                    if !all && (is_dot || name.starts_with('.')) { i += reclen; continue; }
                    names.push((name.len(), String::from(name)));
                }
            }
            i += reclen;
        }
        names.sort_by(|a, b| a.1.cmp(&b.1));
        for (_, name) in &names {
            let entry_path = join_path(path, name);
            if long {
                match libsarga::fs::stat(&entry_path) {
                    Ok(st) => {
                        let mode = st.mode;
                        let size = st.size as u64;
                        let type_char = if (mode & 0o170000) == 0o040000 { 'd' }
                                        else if (mode & 0o170000) == 0o120000 { 'l' }
                                        else { '-' };
                        let perm = |m: u32, r: u32, w: u32, x: u32| -> String {
                            alloc::format!("{}{}{}",
                                if m & r != 0 { "r" } else { "-" },
                                if m & w != 0 { "w" } else { "-" },
                                if m & x != 0 { "x" } else { "-" })
                        };
                        let perm_str = alloc::format!("{}{}{}",
                            perm(mode, 0o400, 0o200, 0o100),
                            perm(mode, 0o040, 0o020, 0o010),
                            perm(mode, 0o004, 0o002, 0o001));

                        let size_str = if human { human_size(size) } else { alloc::format!("{}", size) };
                        print!("{}{} ", type_char, perm_str);
                        print!("{:>8} ", size_str);
                    }
                    Err(_) => {
                        print!("?--------- ???????? ");
                    }
                }
                println!("{}", name);
            } else {
                match libsarga::fs::stat(&entry_path) {
                    Ok(st) => {
                        if (st.mode & 0o170000) == 0o040000 {
                            print!("d ");
                        } else {
                            print!("- ");
                        }
                    }
                    Err(_) => {
                        print!("? ");
                    }
                }
                println!("{}", name);
            }
        }
    }
    let _ = io::close(fd);
    0
    0
}

sarga_main!(user_main);
