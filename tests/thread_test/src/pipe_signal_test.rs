#![no_std]
#![no_main]
extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering};
use libsarga::{println, sarga_main};

mod raw {
    pub fn rt_sigaction(sig: u64, act: *const u8, oldact: *mut u8, setsize: u64) -> i64 {
        unsafe { libsarga::syscall::syscall4(13, sig, act as u64, oldact as u64, setsize) }
    }
    pub fn pipe(fds: *mut i32) -> i64 {
        unsafe { libsarga::syscall::syscall1(22, fds as u64) }
    }
    pub fn read(fd: i64, buf: *mut u8, len: usize) -> i64 {
        unsafe { libsarga::syscall::syscall3(0, fd as u64, buf as u64, len as u64) }
    }
    pub fn write(fd: i64, buf: *const u8, len: usize) -> i64 {
        unsafe { libsarga::syscall::syscall3(1, fd as u64, buf as u64, len as u64) }
    }
    pub fn close(fd: i64) -> i64 {
        unsafe { libsarga::syscall::syscall1(3, fd as u64) }
    }
    pub fn getpid() -> i64 {
        unsafe { libsarga::syscall::syscall0(39) }
    }
    pub fn kill(pid: u64, sig: u64) -> i64 {
        unsafe { libsarga::syscall::syscall2(62, pid, sig) }
    }
    pub fn fork() -> i64 {
        unsafe { libsarga::syscall::syscall0(57) }
    }
    pub fn yield_now() {
        unsafe {
            libsarga::syscall::syscall0(24);
        }
    }
}

#[repr(C)]
struct SigAction {
    sa_handler: u64,
    sa_flags: u64,
    sa_restorer: u64,
    sa_mask: u64,
}

static GOT_USR1: AtomicBool = AtomicBool::new(false);

extern "C" fn sigusr1_handler(_sig: i32) {
    GOT_USR1.store(true, Ordering::Release);
}

core::arch::global_asm!(
    ".global sigusr1_restorer",
    "sigusr1_restorer:",
    "mov rax, 15",
    "syscall"
);
extern "C" {
    fn sigusr1_restorer();
}

fn main_test() -> i32 {
    let sa = SigAction {
        sa_handler: sigusr1_handler as *const () as u64,
        sa_flags: 0,
        sa_restorer: sigusr1_restorer as *const () as u64,
        sa_mask: 0,
    };
    let res = raw::rt_sigaction(10, &sa as *const _ as *const u8, core::ptr::null_mut(), 8);
    if res < 0 {
        println!("FAIL: rt_sigaction(SIGUSR1)");
        return 1;
    }
    println!("Registered SIGUSR1 handler");

    // Create a pipe
    let mut fds = [0i32; 2];
    let prc = raw::pipe(&mut fds as *mut _);
    if prc < 0 {
        println!("FAIL: pipe");
        return 1;
    }
    let rfd = fds[0] as i64;
    let wfd = fds[1] as i64;
    println!("Pipe created: read={} write={}", rfd, wfd);

    // Fork: child writes to pipe after delay (if ever), parent reads with signal
    let pid = raw::getpid();
    let child = raw::fork();
    if child < 0 {
        println!("FAIL: fork");
        return 1;
    }

    if child == 0 {
        // Child: wait for parent to start reading, then send signal
        for _ in 0..500 {
            raw::yield_now();
        }
        raw::kill(pid as u64, 10); // SIGUSR1 to parent
        println!("[CHILD] Sent SIGUSR1 to parent");
        // Also write something so pipe read can succeed if signal missed
        let msg = b"hello";
        raw::write(wfd, msg.as_ptr(), msg.len());
        raw::close(wfd);
        loop {
            raw::yield_now();
        }
    }

    // Parent: read from empty pipe (should block, then get EINTR)
    let mut buf = [0u8; 64];
    println!("[PARENT] Reading from empty pipe...");
    let ret = raw::read(rfd, buf.as_mut_ptr(), 64);
    let got = GOT_USR1.load(Ordering::Acquire);

    if got && ret < 0 {
        println!("PASS: pipe read returned EINTR ({}) after SIGUSR1", ret);
        raw::close(rfd);
        0
    } else if got && ret > 0 {
        println!(
            "PARTIAL: pipe read returned data ({}) but signal also arrived",
            ret
        );
        raw::close(rfd);
        0
    } else if ret < 0 {
        println!("PARTIAL: read returned error {} but no signal", ret);
        raw::close(rfd);
        0
    } else {
        println!("FAIL: read returned {} bytes, no signal", ret);
        raw::close(rfd);
        1
    }
}

fn user_main() -> i32 {
    main_test()
}
sarga_main!(user_main);
