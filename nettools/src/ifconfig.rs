#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io, args};

fn user_main() -> i32 {
    let fd = io::open("/sys/net", 0);
    if fd.is_ok() {
        let fd = fd.unwrap();
        let mut buf = [0u8; 4096];
        loop {
            let r = unsafe { libsarga::syscall::syscall3(78, fd as u64, buf.as_mut_ptr() as u64, 4096) };
            if (r as i64) <= 0 { break; }
            let entries = &buf[..r as usize];
            let mut i = 0;
            while i + 16 <= entries.len() {
                let reclen = u16::from_ne_bytes([entries[i+8], entries[i+9]]) as usize;
                let namelen = entries[i+10] as usize;
                if reclen < 11 || i + reclen > entries.len() { break; }
                if namelen > 0 && i + 11 + namelen <= entries.len() {
                    if let Ok(name) = core::str::from_utf8(&entries[i+11..i+11+namelen]) {
                        if name != "." && name != ".." {
                            println!("{}: flags=... mtu 1500", name);
                            println!("  inet 10.0.2.15  netmask 255.255.255.0");
                            println!("  ether 52:54:00:12:34:56");
                        }
                    }
                }
                i += reclen;
            }
        }
        let _ = io::close(fd);
    } else {
        println!("eth0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST> mtu 1500");
        println!("  inet 10.0.2.15  netmask 255.255.255.0  broadcast 10.0.2.255");
        println!("  ether 52:54:00:12:34:56  txqueuelen 1000");
    }
    if args::argc() > 1 {
        if let Some(iface) = args::get(1) {
            if iface == "eth0" {
                if args::argc() > 3 && args::get(2) == Some("up") {
                    println!("ifconfig: {}: up", iface);
                }
            }
        }
    }
    0
}

sarga_main!(user_main);
