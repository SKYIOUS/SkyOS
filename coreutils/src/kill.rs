#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println};

fn user_main() {
    println!("kill");
    
}
sarga_main!(user_main);
