#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, process, sarga_main, signal};

fn user_main() -> i32 {
    if args::argc() < 2 {
        return 1;
    }
    let _ = signal::signal(signal::SIGHUP, signal::SIG_IGN);
    let cmd = args::get(1).unwrap_or("");
    let mut cmd_args: alloc::vec::Vec<&str> = alloc::vec::Vec::new();
    for i in 1..args::argc() as usize {
        if let Some(a) = args::get(i) {
            cmd_args.push(a);
        }
    }
    match process::execve(cmd, &cmd_args, &[]) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}
sarga_main!(user_main);
