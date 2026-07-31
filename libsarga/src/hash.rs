//! Cryptographic hashing and PBKDF2.

use crate::errno::Error;
use crate::syscall::*;

/// System call number for hashing operations.
pub const SYS_HASH: u64 = 401;
/// Command for PBKDF2-HMAC-SHA256.
pub const HASH_PBKDF2_SHA256: u64 = 0;

/// Compute PBKDF2-HMAC-SHA256.
///
/// `password` is the password bytes. `salt` is the 16-byte salt (e.g. from /etc/shadow).
/// On return, `dk_out` contains the 32-byte derived key.
/// Returns the iteration count actually used.
pub fn pbkdf2_sha256(
    password: &[u8],
    salt: &[u8; 16],
    dk_out: &mut [u8; 32],
    iterations: u32,
) -> Result<u32, Error> {
    let mut buf = [0u8; 48];
    buf[..16].copy_from_slice(salt);
    // SAFETY: hash syscall is safe here
    let r = unsafe {
        syscall5(
            SYS_HASH,
            HASH_PBKDF2_SHA256,
            password.as_ptr() as u64,
            password.len() as u64,
            buf.as_mut_ptr() as u64,
            iterations as u64,
        )
    };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        dk_out.copy_from_slice(&buf[16..48]);
        Ok(r as u32)
    }
}

/// Decode hex string to bytes.
/// Returns None if input is invalid or odd length.
pub fn hex_decode(s: &[u8]) -> Option<alloc::vec::Vec<u8>> {
    hex::decode(s).ok()
}

/// Verify a password against a shadow file entry.
/// 
/// `shadow_data` is the raw bytes of /etc/shadow.
/// `username` is the username to look up.
/// `password` is the password to verify.
/// 
/// Returns true if password matches, false otherwise.
/// Only accepts PBKDF2-HMAC-SHA256 entries for security.
/// Returns false if shadow file is unreadable or user not found.
pub fn verify_password(shadow_data: &[u8], username: &str, password: &str) -> bool {
    for line in shadow_data.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, |&b| b == b':');
        let name = parts.next().unwrap_or(b"");
        if name != username.as_bytes() {
            continue;
        }
        let rest = parts.next().unwrap_or(b"");

        // PBKDF2 format: PBKDF2-<salt-hex>:<dk-hex>[:iterations]
        if rest.starts_with(b"PBKDF2-") {
            let rest2 = &rest[7..];
            let mut parts2 = rest2.splitn(2, |&b| b == b':');
            let salt_hex = parts2.next().unwrap_or(b"");
            let rest3 = parts2.next().unwrap_or(b"");

            let salt_bytes = match hex_decode(salt_hex) {
                Some(s) if s.len() == 16 => s,
                _ => return false,
            };
            let mut salt_arr = [0u8; 16];
            salt_arr.copy_from_slice(&salt_bytes);

            let mut dk_hex = rest3;
            let mut iterations: u32 = 10000;
            if let Some(pos) = rest3.iter().position(|&b| b == b':') {
                dk_hex = &rest3[..pos];
                let iter_str = core::str::from_utf8(&rest3[pos + 1..]).unwrap_or("10000");
                iterations = iter_str.parse().unwrap_or(10000);
            }
            let stored_dk = match hex_decode(dk_hex) {
                Some(s) if s.len() == 32 => s,
                _ => return false,
            };

            let pw = password.as_bytes();
            let mut dk_out = [0u8; 32];
            if pbkdf2_sha256(pw, &salt_arr, &mut dk_out, iterations).is_ok() {
                return dk_out == stored_dk.as_slice();
            }
            return false;
        }
        // Reject non-PBKDF2 entries for security
        return false;
    }
    false
}
