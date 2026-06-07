#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use libsarga::{sarga_main, println, args, syscall::*};

fn user_main() {
    if args::argc() < 2 { println!("Usage: mkdir [-p] <dir>"); return; }
    let mut parent = false;
    let mut dirs: [Option<&str>; 16] = [None; 16];
    let mut count = 0;
    for i in 1..args::argc() {
        if let Some(arg) = args::get(i as usize) {
            if arg == "-p" { parent = true; }
            else if count < 16 { dirs[count] = Some(arg); count += 1; }
        }
    }
    for i in 0..count {
        if let Some(path) = dirs[i] {
            if parent {
                let mut accumulated = String::from("/");
                for segment in path.split('/') {
                    if segment.is_empty() { continue; }
                    if accumulated.len() > 1 { accumulated.push('/'); }
                    accumulated.push_str(segment);
                    unsafe { syscall3(83, accumulated.as_ptr() as u64, 0o755, 0); }
                }
            } else {
                let r = unsafe { syscall3(83, path.as_ptr() as u64, 0o755, 0) };
                if r < 0 { println!("mkdir: failed to create {}", path); }
            }
        }
    }
}
sarga_main!(user_main);
