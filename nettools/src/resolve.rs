#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, net};

fn user_main() -> i32 {
    let name = args::get(1).unwrap_or("");
    if name.is_empty() {
        println!("Usage: resolve <hostname>");
        return 0;
    }
    let mut ip = [0u8; 4];
    match net::resolve(name, &mut ip) {
        Ok(()) => { println!("{} resolved to {}.{}.{}.{}", name, ip[0], ip[1], ip[2], ip[3]); 0 }
        Err(e) => { println!("resolve: lookup failed: {}", e); 1 }
    }
}

sarga_main!(user_main);
