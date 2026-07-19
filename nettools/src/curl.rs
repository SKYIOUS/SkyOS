#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use libsarga::{args, io, net, println, sarga_main};

fn resolve_host(host: &str) -> Option<[u8; 4]> {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
        return Some([
            parts[0].parse().unwrap_or(0),
            parts[1].parse().unwrap_or(0),
            parts[2].parse().unwrap_or(0),
            parts[3].parse().unwrap_or(0),
        ]);
    }
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
    let rest = if let Some(r) = url.strip_prefix("http://") {
        r
    } else {
        url
    };

    let (host_str, path) = if let Some(idx) = rest.find('/') {
        (&rest[..idx], &rest[idx..])
    } else {
        (rest, "/")
    };

    let (host, port) = if host_str.starts_with('[') {
        let close = host_str.find(']').unwrap_or(host_str.len() - 1);
        let addr = &host_str[1..close];
        let p = host_str[close + 1..]
            .strip_prefix(':')
            .and_then(|s| s.parse().ok())
            .unwrap_or(80);
        (addr, p)
    } else {
        let parts: Vec<&str> = host_str.split(':').collect();
        (
            parts[0],
            if parts.len() > 1 {
                parts[1].parse().unwrap_or(80)
            } else {
                80
            },
        )
    };

    if host_str.starts_with('[') {
        let close = host_str.find(']').unwrap_or(host_str.len() - 1);
        let raw = &host_str[1..close];
        match net::parse_ipv6(raw) {
            Some(ip6) => {
                let fd = match net::socket(net::AF_INET6, net::SOCK_STREAM, 0) {
                    Ok(f) => f,
                    Err(e) => {
                        println!("curl: IPv6 socket: {}", e);
                        return 1;
                    }
                };
                let addr = net::SockAddrIn6::new(ip6, port);
                match net::connect(fd, addr.as_bytes()) {
                    Ok(_) => {
                        let request = alloc::format!(
                            "GET {} HTTP/1.1\r\nHost: [{}]\r\nConnection: close\r\n\r\n",
                            path,
                            raw
                        );
                        let _ = net::send(fd, request.as_bytes());
                        let mut buf = [0u8; 4096];
                        loop {
                            match net::recv(fd, &mut buf) {
                                Ok(0) => break,
                                Ok(n) => {
                                    io::write_all(1, &buf[..n]).ok();
                                }
                                Err(_) => break,
                            }
                        }
                        let _ = io::close(fd);
                        return 0;
                    }
                    Err(e) => {
                        println!("curl: IPv6 connect failed: {}", e);
                        return 1;
                    }
                }
            }
            None => {
                println!("curl: invalid IPv6 address");
                return 1;
            }
        }
    }

    if host_str.starts_with('[') {
        return 0;
    }

    let ip = match resolve_host(host) {
        Some(ip) => ip,
        None => {
            println!("curl: could not resolve {}", host);
            return 0;
        }
    };

    match net::socket(net::AF_INET, net::SOCK_STREAM, 0) {
        Ok(fd) => {
            let addr = net::SockAddrIn::new(ip, port);
            match net::connect(fd, addr.as_bytes()) {
                Ok(_) => {
                    let request = alloc::format!(
                        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                        path,
                        host
                    );
                    let _ = net::send(fd, request.as_bytes());
                    let mut buf = [0u8; 4096];
                    loop {
                        match net::recv(fd, &mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                io::write_all(1, &buf[..n]).ok();
                            }
                            Err(_) => break,
                        }
                    }
                    let _ = io::close(fd);
                    return 0;
                }
                Err(e) => {
                    println!("curl: connect failed: {}", e);
                    return 1;
                }
            }
        }
        Err(e) => {
            println!("curl: socket failed: {}", e);
            return 1;
        }
    }
}

sarga_main!(user_main);
