#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::{sarga_main, println, args, syscall};

fn user_main() {
    if args::argc() < 2 { println!("Usage: which <command>..."); return; }
    let path_str = String::from("/bin");
    let dirs: Vec<&str> = path_str.split(':').collect();
    for i in 1..args::argc() {
        if let Some(cmd) = args::get(i as usize) {
            let mut found = false;
            for dir in &dirs {
                let full_path = if dir.ends_with('/') {
                    alloc::format!("{}{}", dir, cmd)
                } else {
                    alloc::format!("{}/{}", dir, cmd)
                };
                let mut st = [0u64; 32];
                if unsafe { syscall::syscall2(4, full_path.as_ptr() as u64, st.as_mut_ptr() as u64) } == 0 {
                    println!("{}", full_path);
                    found = true;
                    break;
                }
            }
            if !found { println!("{} not found", cmd); }
        }
    }
}

sarga_main!(user_main);
