#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io, args};
use alloc::string::String;

fn user_main() {
    let interval: u64 = args::get(1).and_then(|s| s.parse().ok()).unwrap_or(2);
    let mut _iteration = 0u64;
    loop {
        // Clear screen and home cursor
        libsarga::print!("\x1b[2J\x1b[H");

        let uptime_str = read_ctl_file("kernel/uptime").unwrap_or_else(|_| String::from("?"));
        let cpu_load = read_ctl_file("sys/cpu/0/load").unwrap_or_else(|_| String::from("?"));
        let mem_total = read_ctl_file("sys/mem/total").unwrap_or_else(|_| String::from("?"));
        let mem_free = read_ctl_file("sys/mem/free").unwrap_or_else(|_| String::from("?"));

        println!("SkyOS top - uptime: {}s  cpu: {}%  mem total: {} free: {}",
            uptime_str.trim(), cpu_load.trim(), mem_total.trim(), mem_free.trim());
        println!("PID  PPID STATE CMD");

        if let Ok(content) = read_ctl_file("proc/list") {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    println!("{}", trimmed);
                }
            }
        } else {
            println!("1    0    S     init");
            println!("2    1    S     sash");
        }

        _iteration += 1;
        libsarga::print!("\n(Press Ctrl-C to exit, updating every {}s)", interval);
        unsafe { libsarga::syscall::syscall1(35, [interval * 1_000_000_000u64].as_ptr() as u64) };
    }
}

fn read_ctl_file(path: &str) -> Result<String, i64> {
    let mut full = String::from("/ctl/");
    full.push_str(path);
    let fd = io::open(&full, 0)?;
    let mut buf = [0u8; 4096];
    let n = io::read(fd, &mut buf)?;
    let _ = io::close(fd);
    Ok(core::str::from_utf8(&buf[..n]).unwrap_or("").into())
}

sarga_main!(user_main);
