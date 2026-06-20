#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use libsarga::{sarga_main, println, io, net, args};

fn user_main() -> i32 {
    if args::argc() < 3 {
        println!("Usage: nc <host> <port>");
        return 0;
    }
    let host = args::get(1).unwrap_or("10.0.2.2");
    let port_str = args::get(2).unwrap_or("80");
    let port: u16 = port_str.parse().unwrap_or(80);

    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() != 4 { println!("nc: bad address"); return 0; }
    let ip: [u8; 4] = [
        parts[0].parse().unwrap_or(10),
        parts[1].parse().unwrap_or(0),
        parts[2].parse().unwrap_or(2),
        parts[3].parse().unwrap_or(2),
    ];

    match net::socket(net::AF_INET, net::SOCK_STREAM, 0) {
        Ok(fd) => {
            let addr = net::SockAddrIn::new(ip, port);
            match net::connect(fd, addr.as_bytes()) {
                Ok(_) => {
                    println!("nc: connected to {}:{}", host, port);
                    let mut buf = [0u8; 1024];
                    loop {
                        let n = match io::read(0, &mut buf) {
                            Ok(0) => break,
                            Ok(n) => n,
                            Err(_) => break,
                        };
                        let r = net::send(fd, &buf[..n]).unwrap_or(0);
                        if r == 0 { break; }
                        let mut resp = [0u8; 4096];
                        match net::recv(fd, &mut resp) {
                            Ok(0) => break,
                            Ok(n) => { io::write_all(1, &resp[..n]).ok(); }
                            Err(_) => break,
                        }
                    }
                    let _ = net::close(fd);
                    return 0;
                }
                Err(e) => { println!("nc: connect failed: {}", e); return 1; }
            }
        }
        Err(e) => { println!("nc: socket failed: {}", e); return 1; }
    }
}

sarga_main!(user_main);
