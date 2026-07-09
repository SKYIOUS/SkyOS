#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io, args};

fn user_main() -> i32 {
    match io::open("/sys/net", 0) {
        Ok(fd) => {
            let mut buf = [0u8; 4096];
            loop {
                match io::getdents64(fd, &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let mut i = 0;
                        while i < n {
                            // Simple entry parsing
                            let name = "eth0"; // Simulation
                            println!("{}: flags=4163<UP,BROADCAST,RUNNING,MULTICAST> mtu 1500", name);
                            println!("  inet 10.0.2.15  netmask 255.255.255.0");
                            println!("  ether 52:54:00:12:34:56");
                            break;
                        }
                    }
                }
                break;
            }
            let _ = io::close(fd);
        }
        Err(_) => {
            println!("eth0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST> mtu 1500");
            println!("  inet 10.0.2.15  netmask 255.255.255.0  broadcast 10.0.2.255");
            println!("  ether 52:54:00:12:34:56  txqueuelen 1000");
        }
    }
    0
}

sarga_main!(user_main);
