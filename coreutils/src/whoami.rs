#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use libsarga::io::{self, open, read, close};
use libsarga::process::geteuid;
use alloc::string::ToString;

fn read_whole_file(path: &str) -> Result<alloc::vec::Vec<u8>, i64> {
    let fd = open(path, 0)?;
    let mut buf = alloc::vec::Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = read(fd, &mut tmp)?;
        if n == 0 { break; }
        buf.extend_from_slice(&tmp[..n]);
    }
    close(fd)?;
    Ok(buf)
}

fn lookup_name_by_uid(uid: u32) -> alloc::string::String {
    if let Ok(data) = read_whole_file("/etc/passwd\0") {
        for line in data.split(|&b: &u8| b == b'\n') {
            if line.is_empty() { continue; }
            let mut parts = line.splitn(4, |&b: &u8| b == b':');
            let name = parts.next().unwrap_or(b"");
            let _pw_passwd = parts.next();
            let uid_str = parts.next().unwrap_or(b"");
            if let Ok(u) = core::str::from_utf8(uid_str).unwrap_or("0").parse::<u32>() {
                if u == uid {
                    return core::str::from_utf8(name).unwrap_or("?").to_string();
                }
            }
        }
    }
    alloc::format!("uid{}", uid)
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let euid = geteuid() as u32;
    let name = lookup_name_by_uid(euid);
    io::print_str(&name);
    io::print_str("\n");
    libsarga::process::exit(0);
}
