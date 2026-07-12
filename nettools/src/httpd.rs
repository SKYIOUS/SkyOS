#![no_std]
#![no_main]
extern crate alloc;
use alloc::format;
use libsarga::{sarga_main, println, net, io, args};

fn user_main() -> i32 {
    let port: u16 = args::get(1).and_then(|s| s.parse().ok()).unwrap_or(80);
    let ip4 = [10, 0, 2, 15];
    let ip6 = net::parse_ipv6("::1").unwrap();

    let fd4 = net::socket(net::AF_INET, net::SOCK_STREAM, 0).unwrap_or(-1);
    let fd6 = net::socket(net::AF_INET6, net::SOCK_STREAM, 0).unwrap_or(-1);

    if fd4 < 0 && fd6 < 0 { println!("httpd: no socket"); return 1; }

    if fd4 >= 0 {
        let addr = net::SockAddrIn::new(ip4, port);
        if net::bind(fd4, addr.as_bytes()).is_err() || net::listen(fd4, 5).is_err() {
            println!("httpd: IPv4 bind/listen failed");
            let _ = io::close(fd4);
        }
    }
    if fd6 >= 0 {
        let addr = net::SockAddrIn6::new(ip6, port);
        if net::bind(fd6, addr.as_bytes()).is_err() || net::listen(fd6, 5).is_err() {
            println!("httpd: IPv6 bind/listen failed");
            let _ = io::close(fd6);
        }
    }

    println!("httpd: listening on port {}", port);

    let mut buf = [0u8; 2048];
    loop {
        let mut addr_buf = [0u8; 32];
        let mut addr_len: u32 = 32;
        let mut client_fd = -1;
        if fd4 >= 0 {
            if let Ok(fd) = net::accept(fd4, &mut addr_buf, &mut addr_len) {
                client_fd = fd;
            }
        }
        if client_fd < 0 && fd6 >= 0 {
            addr_len = 32;
            if let Ok(fd) = net::accept(fd6, &mut addr_buf, &mut addr_len) {
                client_fd = fd;
            }
        }
        if client_fd < 0 { libsarga::posix::sched_yield(); continue; }

        let n = net::recv(client_fd, &mut buf).unwrap_or(0);
        if n > 0 {
            let body = "<!DOCTYPE html><html><body><h1>Hello from SkyOS!</h1></body></html>\r\n";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            let _ = net::send(client_fd, resp.as_bytes());
        }
        let _ = io::close(client_fd);
    }
}

sarga_main!(user_main);
