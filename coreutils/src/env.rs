#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println};

fn user_main() {
    println!("env");
    
}
sarga_main!(user_main);
