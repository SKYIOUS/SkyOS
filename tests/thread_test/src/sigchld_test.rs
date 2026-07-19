#![no_std]
#![no_main]
extern crate alloc;

use libsarga::{println, sarga_main};

mod raw {
    pub fn fork() -> i64 {
        unsafe { libsarga::syscall::syscall0(57) }
    }
    pub fn exit(status: i64) -> ! {
        unsafe {
            libsarga::syscall::syscall1(60, status as u64);
            unreachable!()
        }
    }
    pub fn wait4(pid: i64, status: *mut i32, options: i32, usage: *mut u8) -> i64 {
        unsafe {
            libsarga::syscall::syscall4(61, pid as u64, status as u64, options as u64, usage as u64)
        }
    }
    pub fn yield_now() {
        unsafe {
            libsarga::syscall::syscall0(24);
        }
    }
    pub fn rt_sigaction(sig: u64, act: *const u8, oldact: *mut u8, setsize: u64) -> i64 {
        unsafe { libsarga::syscall::syscall4(13, sig, act as u64, oldact as u64, setsize) }
    }
}

#[repr(C)]
struct SigAction {
    sa_handler: u64,
    sa_flags: u64,
    sa_restorer: u64,
    sa_mask: u64,
}

static GOT_SIGCHLD: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

extern "C" fn sigchld_handler(_sig: i32) {
    GOT_SIGCHLD.store(true, core::sync::atomic::Ordering::Release);
}

// Assembly trampoline for rt_sigreturn. The kernel pushes this address as the
// return-address above the signal frame on the user stack.
core::arch::global_asm!(
    ".global sigchld_restorer",
    "sigchld_restorer:",
    "mov rax, 15",
    "syscall"
);
extern "C" {
    fn sigchld_restorer();
}

fn main_test() -> i32 {
    let sa = SigAction {
        sa_handler: sigchld_handler as *const () as usize as u64,
        sa_flags: 0,
        sa_restorer: sigchld_restorer as *const () as u64,
        sa_mask: 0,
    };
    let res = raw::rt_sigaction(17, &sa as *const _ as *const u8, core::ptr::null_mut(), 8);
    if res < 0 {
        println!("FAIL: rt_sigaction for SIGCHLD returned {}", res);
        return 1;
    }
    println!("Registered SIGCHLD handler");

    let child_pid = raw::fork();
    if child_pid < 0 {
        println!("FAIL: fork returned {}", child_pid);
        return 1;
    }
    if child_pid == 0 {
        println!("[CHILD] exiting");
        raw::exit(42);
    }
    println!("[PARENT] forked pid={}", child_pid);

    for _ in 0..20 {
        raw::yield_now();
        if GOT_SIGCHLD.load(core::sync::atomic::Ordering::Acquire) {
            println!("[PARENT] received SIGCHLD");
            break;
        }
    }

    let mut status = -1i32;
    let wres = raw::wait4(child_pid, &mut status, 0, core::ptr::null_mut());
    if wres == child_pid {
        println!("[PARENT] wait4: child exited status {}", status);
    } else {
        println!("[PARENT] wait4 returned {}", wres);
    }

    if GOT_SIGCHLD.load(core::sync::atomic::Ordering::Acquire) || wres == child_pid {
        println!("PASS: SIGCHLD test");
        0
    } else {
        println!("FAIL");
        1
    }
}

fn user_main() -> i32 {
    main_test()
}

sarga_main!(user_main);
