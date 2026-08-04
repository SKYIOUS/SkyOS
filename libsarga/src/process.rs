//! Process management and control.

use crate::errno::Error;
use crate::syscall::*;

/// Returns the real user ID of the calling process.
pub fn getuid() -> u64 {
    unsafe { syscall0(301) as u64 }
}
/// Returns the effective user ID of the calling process.
pub fn geteuid() -> u64 {
    unsafe { syscall0(305) as u64 }
}
/// Returns the real group ID of the calling process.
pub fn getgid() -> u64 {
    unsafe { syscall0(302) as u64 }
}
/// Returns the effective group ID of the calling process.
pub fn getegid() -> u64 {
    unsafe { syscall0(306) as u64 }
}

/// Sets the real user ID of the calling process.
pub fn setuid(uid: u64) -> Result<(), Error> {
    let r = unsafe { syscall1(303, uid) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}

/// Sets the real group ID of the calling process.
pub fn setgid(gid: u64) -> Result<(), Error> {
    let r = unsafe { syscall1(304, gid) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}

/// Sets a signal handler.
pub fn signal(sig: u64, handler: u64) -> Result<u64, Error> {
    // SYS_RT_SIGACTION = 13, sets handler, 0 for oldact
    let r = unsafe { syscall3(13, sig, 0, handler) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(r as u64)
    }
}

/// Sends a signal to a process.
pub fn kill(pid: i64, sig: u32) -> Result<(), Error> {
    let r = unsafe { syscall2(62, pid as u64, sig as u64) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}

/// Creates a new process by duplicating the calling process.
pub fn fork() -> Result<u64, Error> {
    let r = unsafe { syscall0(57) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(r as u64)
    }
}

/// Terminates the calling process.
pub fn exit(code: i32) -> ! {
    unsafe {
        syscall1(60, code as u64);
    }
    loop {
        core::hint::spin_loop();
    }
}

/// Waits for a child process to change state.
pub fn wait(pid: u64) -> Result<i32, Error> {
    let mut status: i32 = 0;
    let r = unsafe { syscall3(61, pid, (&mut status) as *mut i32 as u64, 0) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(status)
    }
}

/// Waits for any child process to change state and returns (pid, status).
pub fn waitpid(pid: i64, options: i32) -> Result<(u64, i32), Error> {
    let mut status: i32 = 0;
    // SYS_WAIT4 = 61
    let r = unsafe {
        syscall4(
            61,
            pid as u64,
            (&mut status) as *mut i32 as u64,
            options as u64,
            0,
        )
    };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok((r as u64, status))
    }
}

/// Returns the process ID of the calling process.
pub fn getpid() -> u64 {
    unsafe { syscall0(39) as u64 }
}
/// Returns the process ID of the parent of the calling process.
pub fn getppid() -> u64 {
    unsafe { syscall0(110) as u64 }
}

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

/// Set process group ID.
pub fn setpgid(pid: u64, pgid: u64) -> Result<(), Error> {
    let r = unsafe { syscall2(SYS_SETPGID, pid, pgid) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}

/// Get process group ID.
pub fn getpgid(pid: u64) -> Result<u64, Error> {
    let r = unsafe { syscall1(SYS_GETPGID, pid) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(r as u64)
    }
}

/// Get process group ID of calling process.
pub fn getpgrp() -> u64 {
    unsafe { syscall0(SYS_GETPGRP) as u64 }
}

/// Create a new session.
pub fn setsid() -> Result<u64, Error> {
    let r = unsafe { syscall0(SYS_SETSID) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(r as u64)
    }
}

/// Get session ID.
pub fn getsid(pid: u64) -> Result<u64, Error> {
    let r = unsafe { syscall1(SYS_GETSID, pid) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(r as u64)
    }
}

// ── Credentials ───────────────────────────────────────────────────

/// Get real, effective, and saved user IDs.
pub fn getresuid() -> Result<(u32, u32, u32), Error> {
    let mut ruid = 0u32;
    let mut euid = 0u32;
    let mut suid = 0u32;
    let r = unsafe {
        syscall3(
            SYS_GETRESUID,
            &mut ruid as *mut u32 as u64,
            &mut euid as *mut u32 as u64,
            &mut suid as *mut u32 as u64,
        )
    };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok((ruid, euid, suid))
    }
}

/// Set real, effective, and saved user IDs.
pub fn setresuid(ruid: u32, euid: u32, suid: u32) -> Result<(), Error> {
    let r = unsafe { syscall3(SYS_SETRESUID, ruid as u64, euid as u64, suid as u64) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}

/// Get real, effective, and saved group IDs.
pub fn getresgid() -> Result<(u32, u32, u32), Error> {
    let mut rgid = 0u32;
    let mut egid = 0u32;
    let mut sgid = 0u32;
    let r = unsafe {
        syscall3(
            SYS_GETRESGID,
            &mut rgid as *mut u32 as u64,
            &mut egid as *mut u32 as u64,
            &mut sgid as *mut u32 as u64,
        )
    };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok((rgid, egid, sgid))
    }
}

/// Set real, effective, and saved group IDs.
pub fn setresgid(rgid: u32, egid: u32, sgid: u32) -> Result<(), Error> {
    let r = unsafe { syscall3(SYS_SETRESGID, rgid as u64, egid as u64, sgid as u64) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}

/// Get supplementary group list.
pub fn getgroups() -> Result<alloc::vec::Vec<u32>, Error> {
    let r = unsafe { syscall1(SYS_GETGROUPS, 0) };
    if r < 0 {
        return Err(Error::from_i64(r));
    }
    let count = r as i32;
    if count == 0 {
        return Ok(alloc::vec::Vec::new());
    }
    let mut list = alloc::vec::Vec::with_capacity(count as usize);
    let r = unsafe { syscall2(SYS_GETGROUPS, count as u64, list.as_mut_ptr() as u64) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        unsafe {
            list.set_len(count as usize);
        }
        Ok(list)
    }
}

/// Set supplementary group list.
pub fn setgroups(groups: &[u32]) -> Result<(), Error> {
    let r = unsafe { syscall2(SYS_SETGROUPS, groups.len() as u64, groups.as_ptr() as u64) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}

// ── Resource limits ───────────────────────────────────────────────

/// Get resource limits.
pub fn getrlimit(resource: i32, rlim: &mut crate::io::Rlimit) -> Result<(), Error> {
    let r = unsafe {
        syscall2(
            SYS_GETRLIMIT,
            resource as u64,
            rlim as *mut crate::io::Rlimit as u64,
        )
    };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}

/// Set resource limits.
pub fn setrlimit(resource: i32, rlim: &crate::io::Rlimit) -> Result<(), Error> {
    let r = unsafe {
        syscall2(
            SYS_SETRLIMIT,
            resource as u64,
            rlim as *const crate::io::Rlimit as u64,
        )
    };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}

/// Get/set resource limits of another process.
pub fn prlimit64(
    pid: u64,
    resource: i32,
    new: Option<&crate::io::Rlimit>,
    old: Option<&mut crate::io::Rlimit>,
) -> Result<(), Error> {
    let new_ptr = new.map_or(0, |r| r as *const crate::io::Rlimit as u64);
    let old_ptr = old.map_or(0, |r| r as *mut crate::io::Rlimit as u64);
    let r = unsafe { syscall4(SYS_PRLIMIT64, pid, resource as u64, new_ptr, old_ptr) };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}

/// Replaces the current process image with a new process image.
pub fn execve(path: &str, args: &[&str], env: &[&str]) -> Result<(), Error> {
    let mut p = alloc::vec::Vec::from(path.as_bytes());
    p.push(0);

    let mut arg_data: alloc::vec::Vec<alloc::vec::Vec<u8>> =
        alloc::vec::Vec::with_capacity(args.len());
    for a in args {
        let mut v = alloc::vec::Vec::from(a.as_bytes());
        v.push(0);
        arg_data.push(v);
    }

    let mut argv: alloc::vec::Vec<*const u8> = alloc::vec::Vec::with_capacity(args.len() + 1);
    for v in &arg_data {
        argv.push(v.as_ptr());
    }
    argv.push(core::ptr::null());

    let mut env_data: alloc::vec::Vec<alloc::vec::Vec<u8>> =
        alloc::vec::Vec::with_capacity(env.len());
    for e in env {
        let mut v = alloc::vec::Vec::from(e.as_bytes());
        v.push(0);
        env_data.push(v);
    }

    let mut envp: alloc::vec::Vec<*const u8> = alloc::vec::Vec::with_capacity(env.len() + 1);
    for v in &env_data {
        envp.push(v.as_ptr());
    }
    envp.push(core::ptr::null());

    let r = unsafe {
        syscall3(
            59,
            p.as_ptr() as u64,
            argv.as_ptr() as u64,
            envp.as_ptr() as u64,
        )
    };
    if r < 0 {
        Err(Error::from_i64(r))
    } else {
        Ok(())
    }
}
