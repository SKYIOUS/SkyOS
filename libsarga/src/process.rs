use crate::syscall::*;

pub fn fork() -> Result<u64, i64> {
    let r = unsafe { syscall0(57) };
    if r < 0 { Err(-r) } else { Ok(r as u64) }
}
pub fn exit(code: i32) -> ! {
    unsafe { syscall1(60, code as u64); } loop {}
}
pub fn wait(pid: u64) -> Result<i32, i64> {
    let mut status: i32 = 0;
    let r = unsafe { syscall3(61, pid, (&mut status) as *mut i32 as u64, 0) };
    if r < 0 { Err(-r) } else { Ok(status) }
}
pub fn getpid() -> u64 { unsafe { syscall0(39) as u64 } }
pub fn getppid() -> u64 { unsafe { syscall0(110) as u64 } }

pub fn execve(path: &str, args: &[&str], env: &[&str]) -> i64 {
    let mut p = alloc::vec::Vec::from(path.as_bytes()); p.push(0);

    // Reserve capacity first so push() never reallocates after we take pointers
    let mut arg_data: alloc::vec::Vec<alloc::vec::Vec<u8>> = alloc::vec::Vec::new();
    arg_data.reserve(args.len());
    for a in args {
        let mut v = alloc::vec::Vec::from(a.as_bytes()); v.push(0);
        arg_data.push(v);
    }

    // Now build argv from the stable arg_data entries
    let mut argv: alloc::vec::Vec<*const u8> = alloc::vec::Vec::new();
    argv.reserve(args.len() + 1);
    for v in &arg_data {
        argv.push(v.as_ptr());
    }
    argv.push(core::ptr::null());

    // Same for envp
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

    unsafe { syscall3(59, p.as_ptr() as u64, argv.as_ptr() as u64, envp.as_ptr() as u64) }
}
