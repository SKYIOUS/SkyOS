#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::{args, io, net, println, sarga_main};

fn resolve_host(host: &str, port: u16) -> Result<i64, ()> {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
        let ip = [
            parts[0].parse().unwrap(),
            parts[1].parse().unwrap(),
            parts[2].parse().unwrap(),
            parts[3].parse().unwrap(),
        ];
        let fd = net::socket(net::AF_INET, net::SOCK_STREAM, 0).map_err(|_| ())?;
        let addr = net::SockAddrIn::new(ip, port);
        net::connect(fd, addr.as_bytes()).map_err(|_| {
            let _ = io::close(fd);
        })?;
        return Ok(fd);
    }
    let mut ip = [0u8; 4];
    if net::resolve(host, &mut ip).is_ok() {
        let fd = net::socket(net::AF_INET, net::SOCK_STREAM, 0).map_err(|_| ())?;
        let addr = net::SockAddrIn::new(ip, port);
        net::connect(fd, addr.as_bytes()).map_err(|_| {
            let _ = io::close(fd);
        })?;
        return Ok(fd);
    }
    Err(())
}

fn user_main() -> i32 {
    let url = match args::get(1) {
        Some(u) => u,
        None => {
            println!("usage: wget <url>");
            return 1;
        }
    };

    let rest = match url.strip_prefix("http://") {
        Some(r) => r,
        None => {
            println!("wget: only http:// supported");
            return 1;
        }
    };

    let (host_str, path) = if let Some(idx) = rest.find('/') {
        (&rest[..idx], &rest[idx..])
    } else {
        (rest, "/")
    };

    let (host, port) = if host_str.starts_with('[') {
        let close = host_str.find(']').unwrap_or(host_str.len() - 1);
        let addr = &host_str[1..close];
        let rest_after = &host_str[close + 1..];
        let p = if let Some(r) = rest_after.strip_prefix(':') {
            r.parse().unwrap_or(80)
        } else {
            80
        };
        (addr, p)
    } else {
        let parts: Vec<&str> = host_str.split(':').collect();
        let h = parts[0];
        let p = if parts.len() > 1 {
            parts[1].parse().unwrap_or(80)
        } else {
            80
        };
        (h, p)
    };

    let filename = if path == "/" {
        "index.html"
    } else {
        path.rsplit('/').next().unwrap_or("index.html")
    };

    let fd = if host_str.starts_with('[') {
        let close = host_str.find(']').unwrap_or(host_str.len() - 1);
        let raw = &host_str[1..close];
        match net::parse_ipv6(raw) {
            Some(ip6) => {
                let fd = net::socket(net::AF_INET6, net::SOCK_STREAM, 0).unwrap_or(-1);
                if fd < 0 {
                    println!("wget: IPv6 socket failed");
                    return 1;
                }
                let addr = net::SockAddrIn6::new(ip6, port);
                if net::connect(fd, addr.as_bytes()).is_err() {
                    println!("wget: IPv6 connect failed");
                    let _ = io::close(fd);
                    return 1;
                }
                fd
            }
            None => {
                println!("wget: invalid IPv6 address");
                return 1;
            }
        }
    } else {
        match resolve_host(host, port) {
            Ok(fd) => fd,
            Err(_) => {
                println!("wget: could not resolve {}", host);
                return 1;
            }
        }
    };

    let request = alloc::format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        path,
        host
    );
    let _ = net::send(fd, request.as_bytes());

    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match net::recv(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    let _ = io::close(fd);

    if response.len() < 12 {
        println!("wget: empty response");
        return 1;
    }

    let status_line_end = response.windows(2).position(|w| w == b"\r\n").unwrap_or(12);
    let status_line = core::str::from_utf8(&response[..status_line_end]).unwrap_or("HTTP/1.1 0");
    let status_parts: Vec<&str> = status_line.split_whitespace().collect();
    let status_code: u16 = status_parts
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if status_code == 0 || status_code >= 400 {
        let body = String::from_utf8_lossy(&response);
        println!(
            "wget: HTTP {} {}",
            status_code,
            status_parts.get(2).unwrap_or(&"")
        );
        println!("{}", body);
        return if status_code == 0 {
            1
        } else {
            status_code as i32
        };
    }

    if let Some(pos) = response.windows(4).position(|w| w == b"\r\n\r\n") {
        let body = &response[pos + 4..];
        if filename.ends_with(".html") || filename == "index.html" {
            io::write_all(1, body).ok();
        } else {
            match io::open(filename, 0x42) {
                // O_CREAT | O_RDWR
                Ok(out_fd) => {
                    io::write_all(out_fd, body).ok();
                    let _ = io::close(out_fd);
                    println!("wget: saved {} ({} bytes)", filename, body.len());
                }
                Err(_) => {
                    io::write_all(1, body).ok();
                }
            }
        }
    } else if (200..300).contains(&status_code) {
        io::write_all(1, &response).ok();
    }
    0
}

sarga_main!(user_main);
