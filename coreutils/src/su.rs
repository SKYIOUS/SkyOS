#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use alloc::string::ToString;
use libsarga::hash;
use libsarga::io::{self, close, open, read};
use libsarga::process::{execve, geteuid, setgid, setuid};
use libsarga::sarga_main;

fn read_whole_file(path: &str) -> Result<alloc::vec::Vec<u8>, libsarga::errno::Error> {
    let fd = open(path, 0)?;
    let mut buf = alloc::vec::Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = read(fd, &mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    close(fd)?;
    Ok(buf)
}

fn read_line(fd: i64) -> Result<alloc::vec::Vec<u8>, libsarga::errno::Error> {
    let mut buf = alloc::vec::Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = read(fd, &mut byte)?;
        if n == 0 {
            break;
        }
        if byte[0] == b'\n' || byte[0] == b'\r' {
            break;
        }
        buf.push(byte[0]);
    }
    Ok(buf)
}

fn lookup_user(username: &str) -> Option<(u32, u32, alloc::vec::Vec<u8>, alloc::vec::Vec<u8>)> {
    let data = read_whole_file("/etc/passwd\0").ok()?;
    for line in data.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(7, |&b| b == b':');
        let name = parts.next()?;
        if name == username.as_bytes() {
            let _pw_passwd = parts.next()?;
            let uid_str = parts.next()?;
            let gid_str = parts.next()?;
            let _gecos = parts.next()?;
            let home = parts.next()?;
            let shell = parts.next()?;
            let uid = core::str::from_utf8(uid_str).ok()?.parse::<u32>().ok()?;
            let gid = core::str::from_utf8(gid_str).ok()?.parse::<u32>().ok()?;
            return Some((uid, gid, home.to_vec(), shell.to_vec()));
        }
    }
    None
}

fn verify_password(username: &str, password: &str) -> bool {
    let data = match read_whole_file("/etc/shadow\0") {
        Ok(d) => d,
        Err(_) => return false,
    };
    for line in data.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, |&b| b == b':');
        let name = parts.next().unwrap_or(b"");
        if name != username.as_bytes() {
            continue;
        }
        let rest = parts.next().unwrap_or(b"");
        if rest.starts_with(b"PBKDF2-") {
            let inner = &rest[7..];
            let mut parts2 = inner.splitn(2, |&b| b == b':');
            let salt_hex = parts2.next().unwrap_or(b"");
            let rest3 = parts2.next().unwrap_or(b"");
            let salt_bytes = match hash::hex_decode(salt_hex) {
                Some(s) if s.len() == 16 => s,
                _ => return false,
            };
            let mut salt_arr = [0u8; 16];
            salt_arr.copy_from_slice(&salt_bytes);
            let mut dk_hex = rest3;
            let mut iterations: u32 = 10000;
            if let Some(pos) = rest3.iter().position(|&b| b == b':') {
                dk_hex = &rest3[..pos];
                iterations = core::str::from_utf8(&rest3[pos + 1..])
                    .unwrap_or("10000")
                    .parse()
                    .unwrap_or(10000);
            }
            let stored_dk = match hash::hex_decode(dk_hex) {
                Some(s) if s.len() == 32 => s,
                _ => return false,
            };
            let pw = password.as_bytes();
            let mut dk_out = [0u8; 32];
            if hash::pbkdf2_sha256(pw, &salt_arr, &mut dk_out, iterations).is_ok() {
                return dk_out == stored_dk.as_slice();
            }
            return false;
        }
        return false;
    }
    false
}

fn user_main() -> i32 {
    let argc = libsarga::args::argc();

    if argc < 2 {
        io::print_str("Usage: su [username]\n");
        return 0;
    }

    let target_user = libsarga::args::get(1).unwrap_or("root");
    let (uid, gid, home, shell) = match lookup_user(target_user) {
        Some(v) => v,
        None => {
            io::print_str(&alloc::format!("su: unknown user: {}\n", target_user));
            return 1;
        }
    };

    let euid = geteuid();
    if euid != 0 {
        io::print_str("Password: ");
        let pw_bytes = match read_line(0) {
            Ok(b) => b,
            Err(_) => libsarga::process::exit(1),
        };
        let password = core::str::from_utf8(&pw_bytes).unwrap_or("");
        if !verify_password(target_user, password) {
            io::print_str("\nsu: incorrect password\n");
            return 1;
        }
        io::print_str("\n");
    }

    setgid(gid as u64);
    setuid(uid as u64);

    let shell_name = core::str::from_utf8(&shell).unwrap_or("/bin/sash");
    let home_dir = core::str::from_utf8(&home).unwrap_or("/");

    let env = [
        alloc::format!("HOME={}", home_dir),
        alloc::format!("USER={}", target_user),
        alloc::format!("LOGNAME={}", target_user),
        alloc::format!("SHELL={}", shell_name),
        "TERM=xterm-256color".to_string(),
    ];
    let env_refs: alloc::vec::Vec<&str> = env
        .iter()
        .map(|s: &alloc::string::String| s.as_str())
        .collect();

    execve(shell_name, &[], &env_refs);
    return 1;
}

sarga_main!(user_main);
