#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use libsarga::io::{self, open, close, fchown};
use libsarga::process::getegid;

fn parse_num(s: &str) -> Option<u32> {
    s.parse::<u32>().ok()
}

#[no_mangle]
fn user_main() -> i32 {
    let argc = libsarga::args::argc();

    if argc < 3 {
        io::print_str("Usage: chown <owner>[:<group>] <file>...\n");
        return 0;
    }

    let spec = libsarga::args::get(1).unwrap_or("0");
    let (uid, gid) = if let Some(colon) = spec.find(':') {
        let u = parse_num(&spec[..colon]).unwrap_or(0);
        let g = parse_num(&spec[colon+1..]).unwrap_or(0);
        (u, g)
    } else {
        let u = parse_num(spec).unwrap_or(0);
        (u, getegid() as u32)
    };

    for i in 2..argc as usize {
        let file = match libsarga::args::get(i) {
            Some(f) => f,
            None => continue,
        };
        let path = alloc::format!("{}\0", file);
        let fd = match open(&path, 0) {
            Ok(f) => f,
            Err(e) => {
                io::print_str(&alloc::format!("chown: {}: {}\n", file, e));
                continue;
            }
        };
        let ret = fchown(fd as u64, uid, gid);
        close(fd).ok();
        if ret < 0 {
            io::print_str(&alloc::format!("chown: {}: failed\n", file));
        }
    }
    return 0;
}
