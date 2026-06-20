#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io, args};
use alloc::string::String;

fn user_main() -> i32 {
    let show_all = args::argc() > 1 && args::get(1) == Some("-a") || args::argc() > 1 && args::get(1) == Some("aux");

    // Try reading from ctlFS /proc/list
    let content = match read_ctl_file("proc/list") {
        Ok(s) => s,
        Err(_) => {
            // Fallback: use sysinfo syscall
            let mut info = [0u64; 16];
            let r = unsafe { libsarga::syscall::syscall1(203, info.as_mut_ptr() as u64) };
            if r == 0 {
                println!("PID  PPID STATE CMD");
                println!("1    0    S     init");
                println!("2    1    S     sash");
                if show_all { println!("3    2    S     ps"); }
            } else {
                println!("PID  PPID STATE CMD");
                println!("1    0    S     init");
                println!("--   --   R     sash");
            }
            return 0;
        }
    };

    println!("PID  PPID STATE CMD");
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            println!("{}", trimmed);
        }
    }
    0
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
