use crate::syscall::*;

pub const SYS_HASH: u64 = 401;
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
) -> Result<u32, i64> {
    let mut buf = [0u8; 48];
    buf[..16].copy_from_slice(salt);
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
        Err(-r)
    } else {
        dk_out.copy_from_slice(&buf[16..48]);
        Ok(r as u32)
    }
}
