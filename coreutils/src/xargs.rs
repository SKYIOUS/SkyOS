#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::{sarga_main, println, args, io, syscall};

fn user_main() -> i32 {
    if args::argc() < 2 { println!("Usage: xargs <command> [args...]"); return 0; }
    let mut cmd_args = Vec::new();
    for i in 1..args::argc() {
        if let Some(s) = args::get(i as usize) {
            cmd_args.push(String::from(s));
        }
    }
    let mut buf = [0u8; 4096];
    let mut input = String::new();
    loop {
        let n = match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        if let Ok(s) = core::str::from_utf8(&buf[..n]) {
            input.push_str(s);
        }
    }
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        let mut child_args = cmd_args.clone();
        child_args.push(String::from(trimmed));
        let pid = unsafe { syscall::syscall0(57) } as i64;
        if pid == 0 {
            let mut arg_ptrs = alloc::vec::Vec::new();
            for a in &child_args {
                arg_ptrs.push(a.as_ptr() as u64);
            }
            arg_ptrs.push(0);
            unsafe { syscall::syscall1(59, arg_ptrs.as_ptr() as u64); }
            unsafe { syscall::syscall1(60, 0); }
        } else {
            unsafe { syscall::syscall2(61, pid as u64, 0u64); }
        }
    }
    0
    0
}

sarga_main!(user_main);
