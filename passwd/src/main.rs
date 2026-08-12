#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use alloc::string::ToString;
use alloc::vec::Vec;
use libsarga::errno::Error;
use libsarga::io::{self, close, ioctls, open, read};
use libsarga::process::geteuid;
use libsarga::sarga_main;

/// ECHO flag bit in termios `c_lflag` (POSIX).
const ECHO: u32 = 0x8;

/// Termios layout mirrored from the kernel's `sys_ioctl` and from
/// `login/src/main.rs` (repr(C), 4 u32 fields + `c_cc: [u8; 19]`), so the
/// future kernel TCSETS store copies exactly the same span from both
/// consumers.
#[repr(C)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_cc: [u8; 19],
}

/// Disable input echo on `fd` (TCSETS clear ECHO) so the new password typed
/// at the console is not echoed back onto the wire/log. Best-effort: the
/// kernel's TCSETS is currently a no-op returning 0, so this is
/// forward-compatible with a real termios implementation.
///
/// Returns the previous `c_lflag` on success (`Some`) so the caller can
/// restore it; `None` on any ioctl failure (e.g. non-tty fd) — the caller
/// must then skip the restore so a bogus 0 cannot clobber real termios once
/// the kernel implements TCSETS.
fn echo_off(fd: i64) -> Option<u32> {
    let mut t = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_cc: [0; 19],
    };
    // TCGETS first so the other fields (iflag/oflag/cflag) are preserved
    // when we write back — a TCSETS of a zeroed struct would clobber flow
    // control / canonical flags once the kernel implements it.
    if libsarga::io::ioctl(fd, ioctls::TCGETS, &mut t as *mut _ as *mut u8).is_err() {
        return None;
    }
    let saved = t.c_lflag;
    t.c_lflag &= !ECHO;
    if libsarga::io::ioctl(fd, ioctls::TCSETS, &mut t as *mut _ as *mut u8).is_err() {
        return None;
    }
    Some(saved)
}

/// Restore input echo on `fd` to `lflag` (TCSETS). Best-effort; reads the
/// current termios first so untouched fields are preserved.
fn echo_on(fd: i64, lflag: u32) {
    let mut t = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_cc: [0; 19],
    };
    if libsarga::io::ioctl(fd, ioctls::TCGETS, &mut t as *mut _ as *mut u8).is_err() {
        return;
    }
    t.c_lflag = lflag;
    let _ = libsarga::io::ioctl(fd, ioctls::TCSETS, &mut t as *mut _ as *mut u8);
}

/// Read a password with echo off, restoring the previous `c_lflag` on EVERY
/// read_line outcome (line / EOF / Err) — but only if `echo_off` succeeded,
/// mirroring `login::read_password`. EOF returns whatever was read (passwd's
/// read_line has no Option: it breaks on zero bytes).
fn read_password(fd: i64) -> Result<Vec<u8>, Error> {
    let saved = echo_off(fd);
    let r = read_line(fd);
    if let Some(lflag) = saved {
        echo_on(fd, lflag);
    }
    r
}

fn read_line(fd: i64) -> Result<Vec<u8>, Error> {
    let mut buf = Vec::new();
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

fn read_whole_file(path: &str) -> Result<Vec<u8>, Error> {
    let fd = open(path, 0)?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = read(fd, &mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let _ = close(fd);
    Ok(buf)
}

fn hex_encode(bytes: &[u8]) -> Vec<u8> {
    hex::encode(bytes).into_bytes()
}

fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];

    // Try to read from /dev/urandom if available (best entropy source)
    if let Ok(fd) = libsarga::io::open("/dev/urandom", 0) {
        let mut buf = [0u8; 16];
        if libsarga::io::read(fd, &mut buf).is_ok() {
            let _ = libsarga::io::close(fd);
            return buf;
        }
        let _ = libsarga::io::close(fd);
    }

    // Fallback: use clock-based entropy (better than fixed constant)
    // NOTE: This is not cryptographically secure. A proper getrandom syscall should be added.
    let pid = libsarga::process::getpid();
    let time = libsarga::io::clock_gettime(0).unwrap_or((0, 0));

    let mut seed = pid
        .wrapping_mul(0x9E3779B97F4A7C15)
        .wrapping_add(time.0 as u64)
        .wrapping_add(time.1 as u64);

    for s in &mut salt {
        seed = seed.wrapping_mul(0x5DEECE66D).wrapping_add(0xB);
        *s = (seed >> 8) as u8;
    }

    salt
}

fn set_password(username: &str, new_password: &str) -> Result<(), Error> {
    let data = read_whole_file("/etc/shadow")?;

    let salt = generate_salt();
    let pw = new_password.as_bytes();
    let mut dk = [0u8; 32];
    libsarga::hash::pbkdf2_sha256(pw, &salt, &mut dk, 10000)?;

    let salt_enc = hex_encode(&salt);
    let dk_enc = hex_encode(&dk);
    let salt_hex = core::str::from_utf8(&salt_enc).unwrap_or("");
    let dk_hex = core::str::from_utf8(&dk_enc).unwrap_or("");
    let new_line = alloc::format!(
        "{}:PBKDF2-{}:{}:10000:0:99999:7:::\n",
        username,
        salt_hex,
        dk_hex
    );

    let mut out = Vec::new();
    for line in data.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, |&b| b == b':');
        let name = parts.next().unwrap_or(b"");
        if name == username.as_bytes() {
            out.extend_from_slice(new_line.as_bytes());
        } else {
            out.extend_from_slice(line);
            out.push(b'\n');
        }
    }

    let fd = open("/etc/shadow", 0x41)?;
    libsarga::io::write_all(fd, &out)?;
    let _ = close(fd);
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
    let pw1 = match read_password(0) {
        Ok(b) => core::str::from_utf8(&b).unwrap_or("").to_string(),
        Err(_) => libsarga::process::exit(1),
    };
    io::print_str("Retype new password: ");
    let pw2 = match read_password(0) {
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
        Err(e) => {
            io::print_str(&alloc::format!("passwd: update failed: {}\n", e));
            return 1;
        }
    }
    0
}

sarga_main!(user_main);
