#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use libsarga::{io, net, sarga_main};

fn user_main() -> i32 {
    // ponytail: minimal DHCP client — sends discover, parses offer
    let fd = match net::socket(net::AF_INET, net::SOCK_DGRAM, 0) {
        Ok(f) => f,
        Err(_) => return 1,
    };
    let addr = net::SockAddrIn::new([0, 0, 0, 0], 68);
    let _ = net::bind(fd, &addr);

    let mut msg = Vec::new();
    msg.extend_from_slice(b"\x01\x01\x06\x00"); // op, htype, hlen, hops
    msg.extend_from_slice(&[0u8; 4]); // xid
    msg.extend_from_slice(&[0u8; 4]); // secs, flags
    msg.extend_from_slice(&[0u8; 4]); // ciaddr
    msg.extend_from_slice(&[0u8; 4]); // yiaddr
    msg.extend_from_slice(&[0u8; 4]); // siaddr
    msg.extend_from_slice(&[0u8; 4]); // giaddr
    msg.extend_from_slice(&[0u8; 16]); // chaddr
    msg.extend_from_slice(&[0u8; 64]); // sname
    msg.extend_from_slice(&[0u8; 128]); // file
    msg.extend_from_slice(b"\x63\x82\x53\x63"); // magic cookie
    msg.extend_from_slice(b"\x35\x01\x01"); // DHCP discover
    msg.extend_from_slice(b"\xff"); // end

    let _ = net::sendto(fd, &msg, &net::SockAddrIn::new([255, 255, 255, 255], 67));
    io::print_str("[dhcp] discover sent\n");

    let mut buf = [0u8; 1024];
    match net::recvfrom(fd, &mut buf) {
        Ok(n) if n > 0 => {
            if n > 240 && buf[0] == 0x02 {
                io::print_str("[dhcp] offer received\n");
                let ip = [buf[16], buf[17], buf[18], buf[19]];
                io::print_str(&alloc::format!("[dhcp] offered: {}.{}.{}.{}\n", ip[0], ip[1], ip[2], ip[3]));
            }
        }
        _ => io::print_str("[dhcp] no response\n"),
    }
    0
}

sarga_main!(user_main);
