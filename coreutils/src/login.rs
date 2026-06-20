#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use libsarga::io::{self, open, read, close};
use libsarga::process::{setuid, setgid, execve};
use libsarga::sarga_main;
use alloc::string::ToString;

const PASSWD_PATH: &str = "/etc/passwd\0";
const SHADOW_PATH: &str = "/etc/shadow\0";

fn read_line(fd: i64) -> Result<alloc::vec::Vec<u8>, i64> {
    let mut buf = alloc::vec::Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = read(fd, &mut byte)?;
        if n == 0 { break; }
        if byte[0] == b'\n' || byte[0] == b'\r' { break; }
        buf.push(byte[0]);
    }
    Ok(buf)
}

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

fn lookup_user(username: &str) -> Option<(u32, u32, alloc::vec::Vec<u8>, alloc::vec::Vec<u8>)> {
    let data = read_whole_file(PASSWD_PATH).ok()?;
    for line in data.split(|&b| b == b'\n') {
        if line.is_empty() { continue; }
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

fn hex_nibble(v: u8) -> u8 {
    if v < 10 { b'0' + v } else { b'a' + v - 10 }
}

fn hex_decode(s: &[u8]) -> Option<alloc::vec::Vec<u8>> {
    if s.len() % 2 != 0 { return None; }
    let mut out = alloc::vec::Vec::with_capacity(s.len() / 2);
    for chunk in s.chunks(2) {
        let hi = (chunk[0] as char).to_digit(16)? as u8;
        let lo = (chunk[1] as char).to_digit(16)? as u8;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn verify_password(username: &str, password: &str) -> bool {
    let data = match read_whole_file(SHADOW_PATH) {
        Ok(d) => d,
        Err(_) => return false,
    };
    for line in data.split(|&b| b == b'\n') {
        if line.is_empty() { continue; }
        let mut parts = line.splitn(2, |&b| b == b':');
        let name = parts.next().unwrap_or(b"");
        if name == username.as_bytes() {
            let rest = parts.next().unwrap_or(b"");

            // Legacy: PLAINTEXT-<hex>
            if rest.starts_with(b"PLAINTEXT-") {
                let mut attempt_hex = alloc::vec::Vec::new();
                for &b in password.as_bytes() {
                    attempt_hex.push(hex_nibble(b >> 4));
                    attempt_hex.push(hex_nibble(b & 0xf));
                }
                return &rest[9..] == attempt_hex.as_slice();
            }

            // PBKDF2 format: PBKDF2-<salt-hex>:<dk-hex>[:iterations]
            if rest.starts_with(b"PBKDF2-") {
                let rest2 = &rest[7..];
                let mut parts2 = rest2.splitn(2, |&b| b == b':');
                let salt_hex = parts2.next().unwrap_or(b"");
                let rest3 = parts2.next().unwrap_or(b"");

                // Decode salt
                let salt_bytes = match hex_decode(salt_hex) {
                    Some(s) if s.len() == 16 => s,
                    _ => return false,
                };
                let mut salt_arr = [0u8; 16];
                salt_arr.copy_from_slice(&salt_bytes);

                // Parse remaining: dk-hex or dk-hex:iterations
                let mut dk_hex = rest3;
                let mut iterations: u32 = 10000;
                if let Some(pos) = rest3.iter().position(|&b| b == b':') {
                    dk_hex = &rest3[..pos];
                    let iter_str = core::str::from_utf8(&rest3[pos+1..]).unwrap_or("10000");
                    iterations = iter_str.parse().unwrap_or(10000);
                }
                let stored_dk = match hex_decode(dk_hex) {
                    Some(s) if s.len() == 32 => s,
                    _ => return false,
                };

                // Compute hash and compare
                let pw = password.as_bytes();
                let mut dk_out = [0u8; 32];
                if libsarga::hash::pbkdf2_sha256(pw, &salt_arr, &mut dk_out, iterations).is_ok() {
                    return dk_out == stored_dk.as_slice();
                }
                return false;
            }

            return false;
        }
    }
    false
}

fn user_main() -> i32 {
    let argc = libsarga::args::argc();

    let username = if argc > 1 {
        libsarga::args::get(1).unwrap_or("root").to_string()
    } else {
        io::print_str("login: ");
        let name_bytes = match read_line(0) {
            Ok(b) => b,
            Err(_) => libsarga::process::exit(1),
        };
        if name_bytes.is_empty() { return 1; }
        core::str::from_utf8(&name_bytes).unwrap_or("root").to_string()
    };

    let (uid, gid, _home, _shell) = match lookup_user(&username) {
        Some(v) => v,
        None => {
            io::print_str("login: unknown user\n");
            return 1;
        }
    };

    io::print_str("Password: ");
    let pw_bytes = match read_line(0) {
        Ok(b) => b,
        Err(_) => libsarga::process::exit(1),
    };
    let password = core::str::from_utf8(&pw_bytes).unwrap_or("");

    if !verify_password(&username, password) {
        io::print_str("\nLogin incorrect\n");
        return 1;
    }

    io::print_str("\n");
    setuid(uid as u64);
    setgid(gid as u64);

    let shell_name = core::str::from_utf8(&_shell).unwrap_or("/bin/sash");
    let home_dir = core::str::from_utf8(&_home).unwrap_or("/");

    let env = [
        alloc::format!("HOME={}", home_dir),
        alloc::format!("USER={}", username),
        alloc::format!("LOGNAME={}", username),
        alloc::format!("SHELL={}", shell_name),
        "TERM=xterm-256color".to_string(),
    ];
    let env_refs: alloc::vec::Vec<&str> = env.iter().map(|s: &alloc::string::String| s.as_str()).collect();

    execve(shell_name, &[], &env_refs);
    return 1;
}

sarga_main!(user_main);
