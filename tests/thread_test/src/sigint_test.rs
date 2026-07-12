#![no_std]
#![no_main]
extern crate alloc;

use libsarga::{sarga_main, println};
use core::sync::atomic::{AtomicBool, Ordering};

mod raw {
    pub fn rt_sigaction(sig: u64, act: *const u8, oldact: *mut u8, setsize: u64) -> i64 {
        unsafe { libsarga::syscall::syscall4(13, sig, act as u64, oldact as u64, setsize) }
    }
    pub fn sleep(secs: u64) -> i64 {
        unsafe { libsarga::syscall::syscall2(35, secs, 0u64) }
    }
    pub fn getpid() -> i64 {
        unsafe { libsarga::syscall::syscall0(39) }
    }
    pub fn kill(pid: u64, sig: u64) -> i64 {
        unsafe { libsarga::syscall::syscall2(62, pid, sig) }
    }
    pub fn sigprocmask(how: i32, set: *const u64, oldset: *mut u64) -> i64 {
        unsafe { libsarga::syscall::syscall3(309, how as u64, set as u64, oldset as u64) }
    }
}

#[repr(C)]
struct SigAction {
    sa_handler: u64,
    sa_flags: u64,
    sa_restorer: u64,
    sa_mask: u64,
}

static GOT_SIGINT: AtomicBool = AtomicBool::new(false);

extern "C" fn sigint_handler(_sig: i32) {
    GOT_SIGINT.store(true, Ordering::Release);
}

core::arch::global_asm!(
    ".global sigint_restorer",
    "sigint_restorer:",
    "mov rax, 15",
    "syscall"
);
extern "C" { fn sigint_restorer(); }

fn main_test() -> i32 {
    let sa = SigAction {
        sa_handler: sigint_handler as *const () as u64,
        sa_flags: 0,
        sa_restorer: sigint_restorer as *const () as u64,
        sa_mask: 0,
    };
    let res = raw::rt_sigaction(2, &sa as *const _ as *const u8, core::ptr::null_mut(), 8);
    if res < 0 { println!("FAIL: rt_sigaction"); return 1; }
    println!("Registered SIGINT handler, sleeping 5s...");

    let ret = raw::sleep(5);
    let got = GOT_SIGINT.load(Ordering::Acquire);
    if got {
        println!("PASS: SIGINT handler invoked");
        if ret != 0 { println!("PASS: nanosleep returned EINTR ({})", ret); }
        else { println!("NOTE: nanosleep returned 0 (handler ran after)");
            // Also try self-signal test
            let self_pid = raw::getpid() as u64;
            println!("Testing self-signal SIGUSR1...");
            raw::kill(self_pid, 10); // SIGUSR1
            // Signal delivery happens at next syscall boundary
        }
        0
    } else {
        println!("FAIL: SIGINT not received (sleep returned {})", ret);
        1
    }
}

fn user_main() -> i32 { main_test() }
sarga_main!(user_main);
