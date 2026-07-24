#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, io, print, println, sarga_main};

fn sysv_sum(data: &[u8]) -> (u16, usize) {
    let mut s: u32 = 0;
    for &b in data { s = (s + b as u32) & 0xFFFF; }
    let blocks = (data.len() + 511) / 512;
    (s as u16, blocks)
}

fn user_main() -> i32 {
    if args::argc() > 1 {
        let path = args::get(1).unwrap();
        match io::read_to_string(path) {
            Ok(s) => {
                let (cksum, blocks) = sysv_sum(s.as_bytes());
                println!("{} {} {}", cksum, blocks, path);
            }
            Err(_) => { println!("sum: {}: No such file", path); return 1; }
        }
    } else {
        let mut buf = [0u8; 4096]; let mut all = alloc::vec::Vec::new();
        loop { match io::read(0, &mut buf) { Ok(0) => break, Ok(n) => all.extend_from_slice(&buf[..n]), Err(_) => break, } }
        let (cksum, blocks) = sysv_sum(&all);
        println!("{} {}", cksum, blocks);
    }
    0
}
sarga_main!(user_main);