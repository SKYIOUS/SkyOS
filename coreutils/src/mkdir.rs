#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use libsarga::{sarga_main, println, args, syscall::*};

fn user_main() -> i32 {
    if args::argc() < 2 { println!("Usage: mkdir [-p] <dir>"); return 1; }
    let mut parent = false;
    let mut dirs: [Option<&str>; 16] = [None; 16];
    let mut count = 0;
    for i in 1..args::argc() {
        if let Some(arg) = args::get(i as usize) {
            if arg == "-p" { parent = true; }
            else if count < 16 { dirs[count] = Some(arg); count += 1; }
        }
    }
    let mut exit_code = 0;
    for i in 0..count {
        if let Some(path) = dirs[i] {
            let mut path_c = String::from(path);
            path_c.push('\0');
            if parent {
                let mut accumulated = String::new();
                if path.starts_with('/') {
                    accumulated.push('/');
                }
                for (idx, segment) in path.split('/').enumerate() {
                    if segment.is_empty() { continue; }
                    if idx > 0 && !accumulated.ends_with('/') {
                        accumulated.push('/');
                    }
                    accumulated.push_str(segment);
                    let mut acc_c = accumulated.clone();
                    acc_c.push('\0');
                    unsafe { syscall2(83, acc_c.as_ptr() as u64, 0o755); }
                }
            } else {
                let r = unsafe { syscall2(83, path_c.as_ptr() as u64, 0o755) };
                if r < 0 { println!("mkdir: failed to create {}: error {}", path, -r); exit_code = 1; }
            }
        }
    }
    exit_code
    0
    0
}
sarga_main!(user_main);
