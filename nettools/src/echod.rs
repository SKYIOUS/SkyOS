#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, net, args};

fn user_main() {
    let port: u16 = args::get(1).and_then(|s| s.parse().ok()).unwrap_or(7);
    let ip = [10, 0, 2, 15];

    let fd = match net::socket(net::AF_INET, net::SOCK_STREAM, 0) {
        Ok(fd) => fd,
        Err(e) => { println!("echod: socket: {}", e); return; }
    };

    let addr = net::SockAddrIn::new(ip, port);
    if net::bind(fd, addr.as_bytes()).is_err() {
        println!("echod: bind failed");
        let _ = net::close(fd);
        return;
    }

    if net::listen(fd, 5).is_err() {
        println!("echod: listen failed");
        let _ = net::close(fd);
        return;
    }

    println!("echod: listening on {}:{}", "10.0.2.15", port);

    let mut buf = [0u8; 4096];
    loop {
        let mut addr_buf = [0u8; 16];
        let mut addr_len: u32 = 16;
        match net::accept(fd, &mut addr_buf, &mut addr_len) {
            Ok(client_fd) => {
                println!("echod: accepted connection");
                loop {
                    match net::recv(client_fd, &mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let _ = net::send(client_fd, &buf[..n]);
                        }
                        Err(e) => {
                            println!("echod: recv error: {}", e);
                            break;
                        }
                    }
                }
                let _ = net::close(client_fd);
            }
            Err(e) => {
                if e != 11 { // EAGAIN
                    println!("echod: accept error: {}", e);
                }
            }
        }
    }
}

sarga_main!(user_main);
