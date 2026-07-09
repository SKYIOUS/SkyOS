//! Threading and concurrent execution.

use crate::syscall::*;
use crate::errno::Error;

/// Thread creation flags.
pub const CLONE_VM: u64 = 0x00000100;
/// Thread creation flags.
pub const CLONE_FS: u64 = 0x00000200;
/// Thread creation flags.
pub const CLONE_FILES: u64 = 0x00000400;
/// Thread creation flags.
pub const CLONE_SIGHAND: u64 = 0x00000800;
/// Thread creation flags.
pub const CLONE_THREAD: u64 = 0x00010000;

/// Creates a new thread.
///
/// # Safety
/// This is a low-level primitive. Proper stack management is required.
pub unsafe fn clone(flags: u64, stack: *mut u8) -> Result<i64, Error> {
    // SYS_CLONE = 56
    let r = syscall2(56, flags, stack as u64);
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r) }
}

/// Yields the current thread's CPU time.
pub fn yield_now() {
    // SYS_SCHED_YIELD = 24
    let _ = unsafe { syscall0(24) };
}
