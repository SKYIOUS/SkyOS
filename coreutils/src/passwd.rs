#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use libsarga::io::{self, open, read, close};
use libsarga::process::geteuid;
use libsarga::sarga_main;
use alloc::string::ToString;

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

fn hex_nibble(v: u8) -> u8 {
    if v < 10 { b'0' + v } else { b'a' + v - 10 }
}

fn hex_encode(bytes: &[u8]) -> alloc::vec::Vec<u8> {
    let mut out = alloc::vec::Vec::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(hex_nibble(b >> 4));
        out.push(hex_nibble(b & 0xf));
    }
    out
}

fn generate_salt() -> [u8; 16] {
    // Simple deterministic salt.
    let mut salt = [0u8; 16];
    let tick_bytes = (!0u64).wrapping_mul(0x9E3779B97F4A7C15).to_le_bytes();
    salt[..8].copy_from_slice(&tick_bytes);
    salt[8..16].copy_from_slice(&tick_bytes);
    salt[0] = salt[0].wrapping_add(0x42);
    salt[1] = salt[1].wrapping_add(0x57);
    salt
}

fn set_password(username: &str, new_password: &str) -> Result<(), i64> {
    let data = read_whole_file("/etc/shadow\0")?;

    // Generate salt and compute PBKDF2 hash
    let salt = generate_salt();
    let pw = new_password.as_bytes();
    let mut dk = [0u8; 32];
    // Use 10000 iterations for reasonable security
    match libsarga::hash::pbkdf2_sha256(pw, &salt, &mut dk, 10000) {
        Ok(iter) => iter,
        Err(e) => { io::print_str(&alloc::format!("hash failed: {}\n", e)); return Err(e); }
    };

    let salt_enc = hex_encode(&salt);
    let dk_enc = hex_encode(&dk);
    let salt_hex = core::str::from_utf8(&salt_enc).unwrap_or("");
    let dk_hex = core::str::from_utf8(&dk_enc).unwrap_or("");
    let new_line = alloc::format!("{}:PBKDF2-{}:{}:18000:0:99999:7:::\n", username, salt_hex, dk_hex);

    let mut out = alloc::vec::Vec::new();
    for line in data.split(|&b| b == b'\n') {
        if line.is_empty() { continue; }
        let mut parts = line.splitn(2, |&b| b == b':');
        let name = parts.next().unwrap_or(b"");
        if name == username.as_bytes() {
            out.extend_from_slice(new_line.as_bytes());
        } else {
            out.extend_from_slice(line);
            out.push(b'\n');
        }
    }

    let fd = open("/etc/shadow\0", 0x41)?;
    libsarga::io::write_all(fd, &out)?;
    close(fd)?;
    Ok(())
}

fn user_main() -> i32 {
    let argc = libsarga::args::argc();
    let euid = geteuid();

    let target_user = if argc > 1 {
        libsarga::args::get(1).unwrap_or("").to_string()
    } else {
        "root".to_string()
    };

    if target_user.is_empty() || target_user == "-h" || target_user == "--help" {
        io::print_str("Usage: passwd [username]\n");
        return 0;
    }

    if euid != 0 {
        io::print_str("passwd: only root can change passwords\n");
        return 1;
    }

    io::print_str("New password: ");
    let pw1 = match read_line(0) {
        Ok(b) => core::str::from_utf8(&b).unwrap_or("").to_string(),
        Err(_) => libsarga::process::exit(1),
    };
    io::print_str("Retype new password: ");
    let pw2 = match read_line(0) {
        Ok(b) => core::str::from_utf8(&b).unwrap_or("").to_string(),
        Err(_) => libsarga::process::exit(1),
    };
    if pw1 != pw2 {
        io::print_str("\npasswd: passwords do not match\n");
        return 1;
    }
    if pw1.is_empty() {
        io::print_str("\npasswd: password cannot be empty\n");
        return 1;
    }

    match set_password(&target_user, &pw1) {
        Ok(_) => io::print_str("passwd: password updated successfully\n"),
        Err(e) => { io::print_str(&alloc::format!("passwd: update failed: {}\n", e)); return 1; }
    }
    return 0;
}

sarga_main!(user_main);
