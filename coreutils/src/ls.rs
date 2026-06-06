#![no_std]
#![no_main]
use libsarga::{sarga_main, print, println, io};

fn user_main() {
    println!("ls output (placeholder)");
    
}

sarga_main!(user_main);
