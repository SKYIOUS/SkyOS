#![no_std]
#![no_main]
use libsarga::{sarga_main, print, println, syscall::*};

fn user_main() {
    println!("rm output (placeholder)");
    
}

sarga_main!(user_main);
