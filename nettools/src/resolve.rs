#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, net, println, sarga_main};

fn fmt_ipv6(addr: &[u8; 16]) -> alloc::string::String {
    use core::fmt::Write;
    let mut s = alloc::string::String::new();
    for i in 0..8 {
        let val = (addr[i * 2] as u16) << 8 | addr[i * 2 + 1] as u16;
        if i > 0 {
            write!(s, ":").ok();
        }
        write!(s, "{:x}", val).ok();
    }
    s
}

fn user_main() -> i32 {
    let name = args::get(1).unwrap_or("");
    if name.is_empty() {
        println!("Usage: resolve <hostname>");
        return 0;
    }

    if name.contains(':') {
        match net::parse_ipv6(name) {
            Some(ip6) => {
                println!("{} is an IPv6 literal: [{}]", name, fmt_ipv6(&ip6));
                return 0;
            }
            None => {}
        }
    }

    let mut ip = [0u8; 4];
    match net::resolve(name, &mut ip) {
        Ok(()) => {
            println!(
                "{} resolved to {}.{}.{}.{}",
                name, ip[0], ip[1], ip[2], ip[3]
            );
            0
        }
        Err(e) => {
            println!("resolve: lookup failed: {}", e);
            1
        }
    }
}

sarga_main!(user_main);
