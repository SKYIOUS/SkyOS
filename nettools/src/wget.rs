#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use libsarga::{sarga_main, println, net, io, args};

fn user_main() -> i32 {
    let url = match args::get(1) {
        Some(u) => u,
        None => { println!("usage: wget <url>"); return 1; }
    };

    if !url.starts_with("http://") { println!("wget: only http:// supported"); return 1; }
    let rest = &url[7..];
    let (host, path) = match rest.find('/') {
        Some(pos) => (&rest[..pos], &rest[pos..]),
        None => (rest, "/"),
    };

    let mut ip = [0u8; 4];
    if net::resolve(host, &mut ip).is_err() {
        println!("wget: could not resolve {}", host);
        return 1;
    }

    let fd = match net::socket(net::AF_INET, net::SOCK_STREAM, 0) {
        Ok(fd) => fd,
        Err(e) => { println!("wget: socket: {}", e); return 1; }
    };

    let addr = net::SockAddrIn::new(ip, 80);
    if net::connect(fd, addr.as_bytes()).is_err() {
        println!("wget: connection failed");
        let _ = io::close(fd);
        return 1;
    }

    let request = alloc::format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host);
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

    if let Some(pos) = response.windows(4).position(|w| w == b"\r\n\r\n") {
        io::write_all(1, &response[pos + 4..]).ok();
    } else {
        io::write_all(1, &response).ok();
    }
    0
}

sarga_main!(user_main);
