#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, syscall};

fn user_main() -> i32 {
    let mut symbolic = false;
    let mut target = "";
    let mut link_name = "";
    for i in 1..args::argc() {
        if let Some(s) = args::get(i as usize) {
            if s == "-s" { symbolic = true; }
            else if s.starts_with('-') { continue; }
            else if target.is_empty() { target = s; }
            else { link_name = s; }
        }
    }
    if target.is_empty() || link_name.is_empty() {
        println!("Usage: ln [-s] <target> <link>");
        return 1;
    }
    let r = if symbolic {
        unsafe { syscall::syscall2(88, target.as_ptr() as u64, link_name.as_ptr() as u64) }
    } else {
        unsafe { syscall::syscall2(86, target.as_ptr() as u64, link_name.as_ptr() as u64) }
    };
    if r != 0 { println!("ln: failed to create link"); return 1; } 0
    0
}

sarga_main!(user_main);
