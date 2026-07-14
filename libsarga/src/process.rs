//! Process management and control.

use crate::syscall::*;
use crate::errno::Error;

/// Returns the real user ID of the calling process.
pub fn getuid() -> u64 { unsafe { syscall0(301) as u64 } }
/// Returns the effective user ID of the calling process.
pub fn geteuid() -> u64 { unsafe { syscall0(305) as u64 } }
/// Returns the real group ID of the calling process.
pub fn getgid() -> u64 { unsafe { syscall0(302) as u64 } }
/// Returns the effective group ID of the calling process.
pub fn getegid() -> u64 { unsafe { syscall0(306) as u64 } }

/// Sets the real user ID of the calling process.
pub fn setuid(uid: u64) -> Result<(), Error> {
    let r = unsafe { syscall1(303, uid) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Sets the real group ID of the calling process.
pub fn setgid(gid: u64) -> Result<(), Error> {
    let r = unsafe { syscall1(304, gid) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Sets a signal handler.
pub fn signal(sig: u64, handler: u64) -> Result<u64, Error> {
    // SYS_RT_SIGACTION = 13, sets handler, 0 for oldact
    let r = unsafe { syscall3(13, sig, 0, handler) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r as u64) }
}

/// Sends a signal to a process.
pub fn kill(pid: i64, sig: u32) -> Result<(), Error> {
    let r = unsafe { syscall2(62, pid as u64, sig as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Creates a new process by duplicating the calling process.
pub fn fork() -> Result<u64, Error> {
    let r = unsafe { syscall0(57) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r as u64) }
}

/// Terminates the calling process.
pub fn exit(code: i32) -> ! {
    unsafe { syscall1(60, code as u64); } loop {}
}

/// Waits for a child process to change state.
pub fn wait(pid: u64) -> Result<i32, Error> {
    let mut status: i32 = 0;
    let r = unsafe { syscall3(61, pid, (&mut status) as *mut i32 as u64, 0) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(status) }
}

/// Waits for any child process to change state and returns (pid, status).
pub fn waitpid(pid: i64, options: i32) -> Result<(u64, i32), Error> {
    let mut status: i32 = 0;
    // SYS_WAIT4 = 61
    let r = unsafe { syscall4(61, pid as u64, (&mut status) as *mut i32 as u64, options as u64, 0) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok((r as u64, status)) }
}

/// Returns the process ID of the calling process.
pub fn getpid() -> u64 { unsafe { syscall0(39) as u64 } }
/// Returns the process ID of the parent of the calling process.
pub fn getppid() -> u64 { unsafe { syscall0(110) as u64 } }

/// Spawns a new process running the given command (fork + exec).
/// Returns the child PID on success.
pub fn spawn(command: &str) -> Result<u64, Error> {
    match fork() {
        Ok(0) => {
            let _ = execve(command, &[command], &[]);
            exit(1);
        }
        Ok(pid) => Ok(pid),
        Err(e) => Err(e),
    }
}

/// Replaces the current process image with a new process image.
pub fn execve(path: &str, args: &[&str], env: &[&str]) -> Result<(), Error> {
    let mut p = alloc::vec::Vec::from(path.as_bytes()); p.push(0);

    let mut arg_data: alloc::vec::Vec<alloc::vec::Vec<u8>> = alloc::vec::Vec::new();
    arg_data.reserve(args.len());
    for a in args {
        let mut v = alloc::vec::Vec::from(a.as_bytes()); v.push(0);
        arg_data.push(v);
    }

    let mut argv: alloc::vec::Vec<*const u8> = alloc::vec::Vec::new();
    argv.reserve(args.len() + 1);
    for v in &arg_data {
        argv.push(v.as_ptr());
    }
    argv.push(core::ptr::null());

    let mut env_data: alloc::vec::Vec<alloc::vec::Vec<u8>> = alloc::vec::Vec::new();
    env_data.reserve(env.len());
    for e in env {
        let mut v = alloc::vec::Vec::from(e.as_bytes()); v.push(0);
        env_data.push(v);
    }

    let mut envp: alloc::vec::Vec<*const u8> = alloc::vec::Vec::new();
    envp.reserve(env.len() + 1);
    for v in &env_data {
        envp.push(v.as_ptr());
    }
    envp.push(core::ptr::null());

    let r = unsafe { syscall3(59, p.as_ptr() as u64, argv.as_ptr() as u64, envp.as_ptr() as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}
