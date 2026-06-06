#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println};

fn user_main() {
    println!("sort");
    
}
sarga_main!(user_main);
