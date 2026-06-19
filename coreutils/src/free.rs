#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, print, io};

fn read_ctl(path: &str) -> u64 {
    let mut path_buf = alloc::vec::Vec::from(path.as_bytes());
    path_buf.push(0);
    let fd = match io::open(path, 0) {
        Ok(fd) => fd,
        Err(_) => return 0,
    };
    let mut buf = [0u8; 64];
    let n = io::read(fd, &mut buf).unwrap_or(0);
    let _ = io::close(fd);
    let s = core::str::from_utf8(&buf[..n]).unwrap_or("0");
    s.trim().parse().unwrap_or(0)
}

fn user_main() -> i32 {
    let total = read_ctl("/ctl/sys/mem/total");
    let used = read_ctl("/ctl/sys/mem/used");
    let free = read_ctl("/ctl/sys/mem/free");
    let cached = read_ctl("/ctl/sys/mem/cached");
    let total_kb = total / 1024;
    let used_kb = used / 1024;
    let free_kb = free / 1024;
    let cached_kb = cached / 1024;
    println!("              total        used        free      cached");
    print!("Mem:    ");
    print!("{:>10} ", alloc::format!("{} KB", total_kb));
    print!("{:>10} ", alloc::format!("{} KB", used_kb));
    print!("{:>10} ", alloc::format!("{} KB", free_kb));
    println!("{:>10}", alloc::format!("{} KB", cached_kb));

    0
    0
}

sarga_main!(user_main);
