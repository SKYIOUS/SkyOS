#![no_std]
#![no_main]
use libsarga::{sarga_main, print, println};

fn user_main() {
    println!("echo output (placeholder)");
    
}

sarga_main!(user_main);
