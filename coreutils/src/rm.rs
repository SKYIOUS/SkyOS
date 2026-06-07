#![no_std]
#![no_main]
use libsarga::{sarga_main, println, args, syscall::*};

fn user_main() {
    if args::argc() < 2 {
        println!("Usage: rm [-rf] <path>...");
        libsarga::process::exit(1);
    }
    let mut start = 1;
    let mut recursive = false;
    let mut force = false;
    for i in 1..args::argc() {
        if let Some(s) = args::get(i as usize) {
            if s == "-r" || s == "-rf" || s == "-fr" { recursive = true; start = i + 1; }
            else if s == "-f" { force = true; start = i + 1; }
            else if s.starts_with('-') { continue; }
            else { start = i; break; }
        }
    }
    for i in start..args::argc() {
        if let Some(path) = args::get(i as usize) {
            let r = unsafe { syscall1(87, path.as_ptr() as u64) };
            if r != 0 && !force { println!("rm: {}: failed", path); }
        }
    }
}

sarga_main!(user_main);
