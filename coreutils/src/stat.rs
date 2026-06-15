#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::io;
use libsarga::fs;
use libsarga::args;

fn user_main() {
    if args::argc() < 2 {
        io::print_str("Usage: stat <file>\n");
        return;
    }
    let mut i = 1;
    while i < args::argc() {
        let arg = args::get(i as usize).unwrap_or("");
        match fs::stat(arg) {
            Ok(st) => {
                let mode = st.mode;
                let ftype = if mode & 0o170000 == 0o100000 { "regular file" }
                    else if mode & 0o170000 == 0o040000 { "directory" }
                    else if mode & 0o170000 == 0o120000 { "symbolic link" }
                    else if mode & 0o170000 == 0o060000 { "block device" }
                    else if mode & 0o170000 == 0o020000 { "character device" }
                    else if mode & 0o170000 == 0o010000 { "named pipe" }
                    else if mode & 0o170000 == 0o140000 { "socket" }
                    else { "unknown" };
                io::print_str(&alloc::format!(
                    "  File: {}\n  Size: {}     Blocks: {}     Type: {}\n  Mode: {:06o}   Uid: {}   Gid: {}\n",
                    arg, st.size, st.blocks, ftype, mode & 0o7777, st.uid, st.gid
                ));
            }
            Err(e) => {
                io::print_str(&alloc::format!("stat: cannot stat '{}': {}\n", arg, e));
            }
        }
        i += 1;
    }
}

sarga_main!(user_main);
