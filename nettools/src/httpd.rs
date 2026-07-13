#![no_std]
#![no_main]
extern crate alloc;
use alloc::format;
use alloc::vec::Vec;
use libsarga::{sarga_main, println, net, io, args};

fn mime_type(path: &str) -> &str {
    if path.ends_with(".html") || path.ends_with(".htm") { "text/html" }
    else if path.ends_with(".css") { "text/css" }
    else if path.ends_with(".js") { "application/javascript" }
    else if path.ends_with(".png") { "image/png" }
    else if path.ends_with(".jpg") || path.ends_with(".jpeg") { "image/jpeg" }
    else if path.ends_with(".gif") { "image/gif" }
    else if path.ends_with(".svg") { "image/svg+xml" }
    else if path.ends_with(".json") { "application/json" }
    else if path.ends_with(".txt") || path.ends_with(".md") { "text/plain" }
    else { "application/octet-stream" }
}

fn serve(fd: i64, root: &str, raw_path: &str) {
    if raw_path.contains("..") {
        let resp = "HTTP/1.1 403 Forbidden\r\nContent-Length: 9\r\nConnection: close\r\n\r\nForbidden";
        let _ = io::write_all(fd, resp.as_bytes());
        return;
    }
    let path = if raw_path == "/" { "/index.html" } else { raw_path };
    let full = format!("{}{}", root, path);
    match io::stat(&full) {
        Ok(st) if st.mode & 0x8000 != 0 => {
            let mime = mime_type(&full);
            match io::open(&full, 0) {
                Ok(file_fd) => {
                    let hdr = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n",
                        st.size, mime
                    );
                    if io::write_all(fd, hdr.as_bytes()).is_err() {
                        let _ = io::close(file_fd);
                        return;
                    }
                    let mut buf = [0u8; 4096];
                    loop {
                        match io::read(file_fd, &mut buf) {
                            Ok(0) => break,
                            Ok(n) => if io::write_all(fd, &buf[..n]).is_err() { break; },
                            Err(_) => break,
                        }
                    }
                    let _ = io::close(file_fd);
                }
                Err(_) => {
                    let resp = "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 21\r\nConnection: close\r\n\r\nInternal Server Error";
                    let _ = io::write_all(fd, resp.as_bytes());
                }
            }
        }
        _ => {
            let body = format!("<html><body><h1>404 Not Found</h1><p>{}</p></body></html>\r\n", raw_path);
            let resp = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Length: {}\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            let _ = io::write_all(fd, resp.as_bytes());
        }
    }
}

fn user_main() -> i32 {
    let port: u16 = args::get(1).and_then(|s| s.parse().ok()).unwrap_or(80);
    let root = args::get(2).unwrap_or("/www");
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

    println!("httpd: listening on port {}, root {}", port, root);

    let mut buf = [0u8; 4096];
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
            let req = core::str::from_utf8(&buf[..n]).unwrap_or("");
            let req_line = req.lines().next().unwrap_or("");
            let parts: Vec<&str> = req_line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == "GET" {
                serve(client_fd, root, parts[1]);
            } else if parts.len() >= 2 {
                let resp = "HTTP/1.1 501 Not Implemented\r\nContent-Length: 15\r\nConnection: close\r\n\r\nNot Implemented";
                let _ = io::write_all(client_fd, resp.as_bytes());
            }
        }
        let _ = io::close(client_fd);
    }
}

sarga_main!(user_main);
