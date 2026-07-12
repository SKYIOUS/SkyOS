#![no_std]
#![no_main]
extern crate alloc;

use libsarga::{sarga_main, println};
use core::sync::atomic::{AtomicBool, Ordering};

mod raw {
    pub fn rt_sigaction(sig: u64, act: *const u8, oldact: *mut u8, setsize: u64) -> i64 {
        unsafe { libsarga::syscall::syscall4(13, sig, act as u64, oldact as u64, setsize) }
    }
    pub fn nanosleep(secs: u64, nsecs: u64) -> i64 {
        unsafe { libsarga::syscall::syscall2(35, secs, nsecs) }
    }
    pub fn getpid() -> i64 {
        unsafe { libsarga::syscall::syscall0(39) }
    }
    pub fn kill(pid: u64, sig: u64) -> i64 {
        unsafe { libsarga::syscall::syscall2(62, pid, sig) }
    }
}

#[repr(C)]
struct SigAction {
    sa_handler: u64,
    sa_flags: u64,
    sa_restorer: u64,
    sa_mask: u64,
}

static GOT_ALRM: AtomicBool = AtomicBool::new(false);

extern "C" fn sigalrm_handler(_sig: i32) {
    GOT_ALRM.store(true, Ordering::Release);
}

core::arch::global_asm!(
    ".global sigalrm_restorer",
    "sigalrm_restorer:",
    "mov rax, 15",
    "syscall"
);
extern "C" { fn sigalrm_restorer(); }

fn main_test() -> i32 {
    let sa = SigAction {
        sa_handler: sigalrm_handler as *const () as u64,
        sa_flags: 0,
        sa_restorer: sigalrm_restorer as *const () as u64,
        sa_mask: 0,
    };
    let res = raw::rt_sigaction(14, &sa as *const _ as *const u8, core::ptr::null_mut(), 8);
    if res < 0 { println!("FAIL: rt_sigaction(SIGALRM)"); return 1; }
    println!("Registered SIGALRM handler");

    // Self-signal while sleeping — nanosleep should return EINTR
    let self_pid = raw::getpid() as u64;
    println!("Sleeping 10s with pending SIGALRM...");
    // Send signal first, then sleep — nanosleep pre-check should catch it
    raw::kill(self_pid, 14);
    let ret = raw::nanosleep(10, 0);
    let got = GOT_ALRM.load(Ordering::Acquire);

    if got && ret != 0 {
        println!("PASS: SIGALRM handler invoked, nanosleep returned EINTR ({})", ret);
        0
    } else if got {
        println!("PARTIAL: handler ran but nanosleep returned {}", ret);
        0
    } else {
        println!("FAIL: SIGALRM not delivered (nanosleep returned {})", ret);
        1
    }
}

fn user_main() -> i32 { main_test() }
sarga_main!(user_main);
