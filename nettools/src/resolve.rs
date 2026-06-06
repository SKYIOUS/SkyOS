#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, net};

fn user_main() {
    let name = args::get(1).unwrap_or("");
    if name.is_empty() {
        println!("Usage: resolve <hostname>");
        return;
    }
    let mut ip = [0u8; 4];
    match net::resolve(name, &mut ip) {
        Ok(()) => {
            println!("{} resolved to {}.{}.{}.{}", name, ip[0], ip[1], ip[2], ip[3]);
        }
        Err(e) => {
            println!("resolve: lookup failed: {}", e);
        }
    }
}

sarga_main!(user_main);
