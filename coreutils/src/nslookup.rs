#![no_std]
#![no_main]
extern crate alloc;
use libsarga::libskyos::net_ext;
use libsarga::{args, println, sarga_main};

fn user_main() -> i32 {
    let hostname = if args::argc() > 1 {
        args::get(1).unwrap_or("")
    } else {
        println!("Usage: nslookup <hostname>");
        return 0;
    };

    match net_ext::resolve(hostname) {
        Some(ip) => {
            println!("Server:  0.0.0.0");
            println!("Name:    {}", hostname);
            println!("Address: {}", ip);
            0
        }
        None => {
            println!("nslookup: {}: Host not found", hostname);
            1
        }
    }
}

sarga_main!(user_main);
