#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{io, print, println, sarga_main};

fn user_main() -> i32 {
    println!("Active Internet connections");
    if let Ok(data) = io::read_to_string("/proc/net/sockstat") {
        print!("{}", data);
    } else if let Ok(data) = io::read_to_string("/proc/net/if_inet6") {
        print!("{}", data);
    } else {
        println!("Active Internet connections (no info available)");
    }
    0
}

sarga_main!(user_main);
