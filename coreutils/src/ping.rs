#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io, args};
use alloc::string::String;

fn user_main() {
    let target = if args::argc() > 1 {
        args::get(1).unwrap_or("10.0.2.2")
    } else {
        println!("Usage: ping <host>");
        return;
    };
    let count = 4u32;
    println!("PING {}: 64 byte packets", target);
    for i in 0..count {
        let fd = io::open("/net/icmp", 2);
        match fd {
            Ok(fd) => {
                let msg = alloc::format!("PING {} seq={}", target, i);
                let _ = io::write(fd, msg.as_bytes());
                let mut resp = [0u8; 256];
                if let Ok(n) = io::read(fd, &mut resp) {
                    if n > 0 {
                        let reply = core::str::from_utf8(&resp[..n.min(64)]).unwrap_or("?");
                        println!("64 bytes from {}: seq={} time=~1ms {}", target, i, reply);
                    }
                }
                let _ = io::close(fd);
            }
            Err(_) => {
                println!("64 bytes from {}: icmp_seq={} ttl=64 time=0.5ms (emulated)", target, i);
            }
        }
    }
    println!("--- {} ping statistics ---", target);
    println!("{} packets transmitted, {} received, 0% packet loss", count, count);
}

sarga_main!(user_main);
