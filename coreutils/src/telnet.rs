#![no_std]
#![no_main]
extern crate alloc;

use libsarga::libskyos::net_ext;
use libsarga::{args, io, println, sarga_main};

fn user_main() -> i32 {
    if args::argc() < 2 {
        println!("Usage: telnet <host> [port]");
        return 0;
    }

    let host = args::get(1).unwrap_or("");
    let port: u16 = args::get(2).and_then(|s| s.parse().ok()).unwrap_or(23);

    let ip = match net_ext::resolve(host) {
        Some(ip) => ip,
        None => {
            println!("telnet: could not resolve '{}'", host);
            return 1;
        }
    };

    println!("Trying {}...", ip);

    let fd = match net_ext::socket(net_ext::AF_INET, net_ext::SOCK_STREAM, 0) {
        f if f >= 0 => f,
        _ => {
            println!("telnet: socket creation failed");
            return 1;
        }
    };

    let addr = net_ext::SocketAddrV4 { ip, port };
    if net_ext::connect(fd, &addr) != 0 {
        println!("telnet: connect to {}:{} failed", ip, port);
        let _ = io::close(fd);
        return 1;
    }

    println!("Connected to {}.", host);

    let mut buf = [0u8; 4096];
    loop {
        let mut readfds = io::FdSet::new();
        readfds.set(0);
        readfds.set(fd);

        if io::select((fd + 1) as i32, Some(&mut readfds), None, None, None).is_err() {
            break;
        }

        if readfds.is_set(0) {
            match io::read(0, &mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if io::write_all(fd, &buf[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        if readfds.is_set(fd) {
            match io::read(fd, &mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if io::write_all(1, &buf[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }

    let _ = io::close(fd);
    0
}

sarga_main!(user_main);
