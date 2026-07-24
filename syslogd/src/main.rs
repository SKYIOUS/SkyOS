#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::io::{self, open, read, write};
use libsarga::process::{exit, fork, wait};
use libsarga::sarga_main;

const LOG_FIFO: &str = "/var/run/syslog.fifo";
const LOG_FILE: &str = "/var/log/syslog";

fn ensure_fifo() -> Result<i64, ()> {
    match open(LOG_FIFO, 0) {
        Ok(fd) => Ok(fd),
        Err(_) => {
            // Create FIFO via mknod — libsarga doesn't have mknod wrapper
            // ponytail: fallback to temp log mode
            Err(())
        }
    }
}

fn append_log(msg: &str) {
    let ts = "timestamp"; // ponytail: real time formatting when clock_gettime is available
    let line = alloc::format!("[{}] {}\n", ts, msg.trim_end_matches('\n'));
    if let Ok(fd) = open(LOG_FILE, 0o201) {
        // O_WRONLY|O_CREAT|O_APPEND
        let _ = write(fd, line.as_bytes());
        let _ = libsarga::io::close(fd);
    }
}

fn user_main() -> i32 {
    // ponytail: FIFO-based log daemon, poll instead of blocking if needed
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
                // ponytail: sleep and retry
                for _ in 0..1000000 {
                    core::hint::spin_loop();
                }
            }
        }
    }
}

sarga_main!(user_main);
