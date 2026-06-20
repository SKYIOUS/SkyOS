#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::sarga_main;
use libsarga::io::{self, notify, getdents64};

const NOTIFY_DIR: &str = "/tmp/notify_inbox";
const NOTIFY_LOG: &str = "/tmp/notifications.log";

fn read_file(path: &str) -> String {
    let fd = match io::open(path, 0) {
        Ok(f) => f,
        Err(_) => return String::new(),
    };
    let mut result = String::new();
    let mut buf = [0u8; 512];
    loop {
        match io::read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                    result.push_str(s);
                }
            }
            Err(_) => break,
        }
    }
    let _ = io::close(fd);
    result
}

fn write_log(entry: &str) {
    if let Ok(fd) = io::open(NOTIFY_LOG, 0x2 | 0x40 | 0x200) { // O_WRONLY|O_CREAT|O_APPEND
        let _ = io::write_all(fd, entry.as_bytes());
        let _ = io::close(fd);
    }
}

fn list_notify_files() -> Vec<String> {
    let mut files = Vec::new();
    let fd = match io::open(NOTIFY_DIR, 0) {
        Ok(f) => f,
        Err(_) => return files,
    };
    let mut buf = [0u8; 2048];
    loop {
        match getdents64(fd, &mut buf) {
            Ok(n) if n > 0 => {
                let mut offset = 0;
                while offset < n {
                    if offset + 19 > n { break; }
                    let ino = u64::from_ne_bytes(buf[offset..offset + 8].try_into().unwrap_or([0; 8]));
                    let reclen = u16::from_ne_bytes(buf[offset + 16..offset + 18].try_into().unwrap_or([0; 2])) as usize;
                    let name_len = buf[offset + 18] as usize;
                    if reclen == 0 || offset + reclen > n { break; }
                    if ino != 0 && name_len > 0 && name_len < 256 {
                        let name_bytes = &buf[offset + 19..offset + 19 + name_len];
                        let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(name_len);
                        if let Ok(name) = core::str::from_utf8(&name_bytes[..name_end]) {
                            if name != "." && name != ".." {
                                files.push(String::from(name));
                            }
                        }
                    }
                    offset += reclen;
                }
            }
            _ => break,
        }
    }
    let _ = io::close(fd);
    files
}

fn delete_file(path: &str) {
    let _ = io::unlink(path);
}

fn user_main() -> i32 {
    io::print_str("[skyd] notification daemon started\n");

    // Ensure inbox directory exists
    io::mkdir(NOTIFY_DIR, 0o755);

    // Write initial log entry
    write_log("[skyd] daemon started\n");

    loop {
        // Check for notification files
        let files = list_notify_files();
        for name in &files {
            let path = if NOTIFY_DIR.ends_with('/') {
                alloc::format!("{}{}", NOTIFY_DIR, name)
            } else {
                alloc::format!("{}/{}", NOTIFY_DIR, name)
            };

            let content = read_file(&path);
            if !content.is_empty() {
                // Log it
                let entry = alloc::format!("[{}] {}\n", name, content);
                write_log(&entry);

                // Forward to kernel toast
                notify(&content, 3000);

                // Delete the request file
                delete_file(&path);
            }
        }

        // Sleep 500ms
        unsafe { libsarga::syscall::syscall1(35, 500_000_000u64); }
    }
}

sarga_main!(user_main);
