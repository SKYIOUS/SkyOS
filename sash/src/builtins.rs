#![no_std]
use alloc::vec::Vec;
use libsarga::println;

pub fn handle_builtin(parts: &[&str]) -> bool {
    let cmd = parts[0];
    match cmd {
        "cd" => {
            println!("cd: not implemented");
            true
        }
        "pwd" => {
            println!("pwd: not implemented");
            true
        }
        _ => false,
    }
}
