#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, io};

fn user_main() -> i32 {
    let _ = args::argc();
    let mut buf = [0u8; 64];
    let fd = match io::open("/ctl/kernel/uptime", 0) {
        Ok(fd) => fd,
        Err(_) => { println!("uptime: failed to read uptime"); return 0; }
    };
    let n = match io::read(fd, &mut buf) {
        Ok(n) => n,
        Err(_) => { let _ = io::close(fd); println!("uptime: read error"); return 0; }
    };
    let _ = io::close(fd);
    if let Ok(s) = core::str::from_utf8(&buf[..n]) {
        let secs: u64 = s.trim().parse().unwrap_or(0);
        let days = secs / 86400;
        let hrs = (secs % 86400) / 3600;
        let mins = (secs % 3600) / 60;
        let secs_remain = secs % 60;
        if days > 0 {
            println!("up {} day{}, {:02}:{:02}:{:02}", days, if days == 1 { "" } else { "s" }, hrs, mins, secs_remain);
        } else {
            println!("up {:02}:{:02}:{:02}", hrs, mins, secs_remain);
        }
    }
    0
}

sarga_main!(user_main);
