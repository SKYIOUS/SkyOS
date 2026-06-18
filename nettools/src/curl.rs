#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use libsarga::{sarga_main, println, io, net, args};

fn resolve_host(host: &str) -> Option<[u8; 4]> {
    // Try dotted-quad IP first
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
        return Some([
            parts[0].parse().unwrap_or(0),
            parts[1].parse().unwrap_or(0),
            parts[2].parse().unwrap_or(0),
            parts[3].parse().unwrap_or(0),
        ]);
    }
    // Fall back to kernel DNS resolver
    let mut ip = [0u8; 4];
    match net::resolve(host, &mut ip) {
        Ok(()) => Some(ip),
        Err(_) => None,
    }
}

fn user_main() -> i32 {
    if args::argc() < 2 {
        println!("Usage: curl <url>");
        return 0;
    }
    let url = args::get(1).unwrap_or("http://10.0.2.2/");
    let rest = if let Some(r) = url.strip_prefix("http://") { r } else { url };
    let (host, path) = if let Some(idx) = rest.find('/') {
        (&rest[..idx], &rest[idx..])
    } else {
        (rest, "/")
    };
    let ip = match resolve_host(host) {
        Some(ip) => ip,
        None => { println!("curl: could not resolve {}", host); return 0; }
    };

    match net::socket(net::AF_INET, net::SOCK_STREAM, 0) {
        Ok(fd) => {
            let port = 80;
            let addr = net::SockAddrIn::new(ip, port);
            match net::connect(fd, addr.as_bytes()) {
                Ok(_) => {
                    let request = alloc::format!(
                        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                        path, host
                    );
                    let _ = net::send(fd, request.as_bytes());
                    let mut buf = [0u8; 4096];
                    loop {
                        match net::recv(fd, &mut buf) {
                            Ok(0) => break,
                            Ok(n) => { io::write_all(1, &buf[..n]).ok(); }
                            Err(_) => break,
                        }
                    }
                    let _ = net::close(fd);
                    0
                }
                Err(e) => { println!("curl: connect failed: {}", e); 1 }
            }
        }
        Err(e) => { println!("curl: socket failed: {}", e); 1 }
    }
    0
}

sarga_main!(user_main);
