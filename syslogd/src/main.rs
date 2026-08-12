#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use libsarga::io::{open, read, write};
use libsarga::process::fork;
use libsarga::sarga_main;

const LOG_FIFO: &str = "/var/run/syslog.fifo";
const LOG_FILE: &str = "/var/log/syslog";

fn append_log(msg: &str) {
    let ts = get_timestamp();
    let line = alloc::format!("[{}] {}\n", ts, msg.trim_end_matches('\n'));
    if let Ok(fd) = open(LOG_FILE, 0o201) {
        // O_WRONLY|O_CREAT|O_APPEND
        let _ = write(fd, line.as_bytes());
        let _ = libsarga::io::close(fd);
    }
}

fn get_timestamp() -> String {
    let mut tv_sec: i64 = 0;
    unsafe {
        libsarga::syscall::syscall2(228, &mut tv_sec as *mut i64 as u64, 0);
    }
    if tv_sec > 0 {
        let secs = tv_sec as u64;
        let time = secs % 86400;
        let h = time / 3600;
        let m = (time % 3600) / 60;
        let s = time % 60;
        alloc::format!("{:02}:{:02}:{:02}", h, m, s)
    } else {
        alloc::format!("t={}", tv_sec)
    }
}

fn user_main() -> i32 {
    if fork().unwrap_or(0) != 0 {
        return 0;
    }

    let mut buf = [0u8; 4096];
    loop {
        match open(LOG_FIFO, 0) {
            Ok(fd) => {
                loop {
                    let n = read(fd, &mut buf).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                        append_log(s);
                    }
                }
                let _ = libsarga::io::close(fd);
            }
            Err(_) => {
                let _ = libsarga::io::nanosleep(1_000_000_000);
            }
        }
    }
}

sarga_main!(user_main);
