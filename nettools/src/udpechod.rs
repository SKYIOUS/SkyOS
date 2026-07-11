#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io, args, syscall};
use libsarga::libskyos::net_ext::{self, SOCK_DGRAM, AF_INET, SocketAddrV4, Ipv4Addr};

fn user_main() -> i32 {
    let port: u16 = args::get(1).and_then(|s| s.parse().ok()).unwrap_or(7);

    let fd = net_ext::socket(AF_INET, SOCK_DGRAM, 0);
    if fd < 0 {
        println!("udpechod: socket: {}", fd);
        return 0;
    }

    let bind_addr = SocketAddrV4 { ip: Ipv4Addr::new(0, 0, 0, 0), port };
    let mut raw = [0u8; 8];
    raw[..2].copy_from_slice(&(AF_INET as u16).to_be_bytes());
    raw[2..4].copy_from_slice(&bind_addr.port.to_be_bytes());
    raw[4..8].copy_from_slice(&bind_addr.ip.0);
    let ret = unsafe { syscall::syscall3(syscall::SYS_BIND, fd as u64, raw.as_ptr() as u64, 8) };
    if ret != 0 {
        println!("udpechod: bind failed: {}", ret);
        let _ = io::close(fd);
        return 0;
    }

    println!("udpechod: listening on UDP port {}", port);

    let mut buf = [0u8; 2048];
    loop {
        let (n, src) = net_ext::recvfrom(fd, &mut buf);
        if n <= 0 { continue; }
        if let Some(addr) = src {
            println!("udpechod: {} bytes from {}:{}", n, addr.ip, addr.port);
            net_ext::sendto(fd, &buf[..n as usize], &addr);
        }
    }
}

sarga_main!(user_main);
