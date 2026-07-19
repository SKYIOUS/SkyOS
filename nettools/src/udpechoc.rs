#![no_std]
#![no_main]
extern crate alloc;
use libsarga::libskyos::net_ext::{self, Ipv4Addr, SocketAddrV4, AF_INET, SOCK_DGRAM};
use libsarga::{args, io, println, sarga_main};

fn user_main() -> i32 {
    let port: u16 = args::get(1).and_then(|s| s.parse().ok()).unwrap_or(7);
    let target_ip = Ipv4Addr::new(127, 0, 0, 1);

    let fd = net_ext::socket(AF_INET, SOCK_DGRAM, 0);
    if fd < 0 {
        println!("udpechoc: socket: {}", fd);
        return 0;
    }

    let dest = SocketAddrV4 {
        ip: target_ip,
        port,
    };
    let msg = b"Hello UDP echo!";
    println!(
        "udpechoc: sending {} bytes to {}:{}",
        msg.len(),
        target_ip,
        port
    );

    let sent = net_ext::sendto(fd, msg, &dest);
    if sent <= 0 {
        println!("udpechoc: sendto failed: {}", sent);
        let _ = io::close(fd);
        return 0;
    }

    let mut buf = [0u8; 2048];
    let (n, src) = net_ext::recvfrom(fd, &mut buf);
    if n <= 0 {
        println!("udpechoc: recvfrom failed or timed out: {}", n);
        let _ = io::close(fd);
        return 1;
    }

    if let Some(addr) = src {
        println!(
            "udpechoc: received {} bytes from {}:{}",
            n, addr.ip, addr.port
        );
    }

    if n as usize == msg.len() && &buf[..n as usize] == msg {
        println!("udpechoc: SUCCESS - data matches!");
    } else {
        println!(
            "udpechoc: FAIL - data mismatch (got {} bytes, expected {})",
            n,
            msg.len()
        );
        let _ = io::close(fd);
        return 1;
    }

    let _ = io::close(fd);
    0
}

sarga_main!(user_main);
