#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use libsarga::io;
use libsarga::net::{self, socket, SockAddrIn};
use libsarga::sarga_main;

fn user_main() -> i32 {
    // ponytail: minimal DHCP client using raw AF_INET/SOCK_DGRAM socket
    let fd = match socket(net::AF_INET, 2, 0) {
        // 2 = SOCK_DGRAM
        Ok(f) => f,
        Err(_) => return 1,
    };
    let addr = SockAddrIn::new([0, 0, 0, 0], 68);
    let _ = libsarga::net::bind(fd, addr.as_bytes());

    let mut msg = Vec::new();
    msg.extend_from_slice(b"\x01\x01\x06\x00");
    msg.extend_from_slice(&[0u8; 4]);
    msg.extend_from_slice(&[0u8; 4]);
    msg.extend_from_slice(&[0u8; 4]);
    msg.extend_from_slice(&[0u8; 4]);
    msg.extend_from_slice(&[0u8; 4]);
    msg.extend_from_slice(&[0u8; 4]);
    msg.extend_from_slice(&[0u8; 16]);
    msg.extend_from_slice(&[0u8; 64]);
    msg.extend_from_slice(&[0u8; 128]);
    msg.extend_from_slice(b"\x63\x82\x53\x63");
    msg.extend_from_slice(b"\x35\x01\x01");
    msg.extend_from_slice(b"\xff");

    let bcast = SockAddrIn::new([255, 255, 255, 255], 67);
    // ponytail: raw syscall for sendto since libsarga doesn't wrap it
    unsafe {
        libsarga::syscall::syscall6(44, fd as u64, msg.as_ptr() as u64, msg.len() as u64, 0, bcast.as_bytes().as_ptr() as u64, bcast.as_bytes().len() as u64);
    }
    io::print_str("[dhcp] discover sent\n");

    let mut buf = [0u8; 1024];
    let n = libsarga::io::read(fd, &mut buf).unwrap_or(0);
    if n > 240 && buf[0] == 0x02 {
        io::print_str("[dhcp] offer received\n");
        let ip = [buf[16], buf[17], buf[18], buf[19]];
        io::print_str(&alloc::format!("[dhcp] offered: {}.{}.{}.{}\n", ip[0], ip[1], ip[2], ip[3]));
    } else {
        io::print_str("[dhcp] no response\n");
    }
    0
}

sarga_main!(user_main);
