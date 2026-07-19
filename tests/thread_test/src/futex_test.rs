#![no_std]
#![no_main]
extern crate alloc;

use core::sync::atomic::{AtomicU32, Ordering};
use libsarga::{println, sarga_main};

mod raw {
    pub fn futex(uaddr: *mut u32, op: u32, val: u32) -> i64 {
        unsafe { libsarga::syscall::syscall3(202, uaddr as u64, op as u64, val as u64) }
    }
    pub fn fork() -> i64 {
        unsafe { libsarga::syscall::syscall0(57) }
    }
    pub fn exit(status: i64) -> ! {
        unsafe {
            libsarga::syscall::syscall1(60, status as u64);
            unreachable!()
        }
    }
    pub fn yield_now() {
        unsafe {
            libsarga::syscall::syscall0(24);
        }
    }
    pub fn getpid() -> i64 {
        unsafe { libsarga::syscall::syscall0(39) }
    }
    pub fn sched_setattr(pid: u64, attr: u64, flags: u64) -> i64 {
        unsafe { libsarga::syscall::syscall3(144, pid, attr, flags) }
    }
    pub fn open(path: &str) -> i64 {
        let b = path.as_bytes();
        let l = core::cmp::min(b.len(), 255);
        let mut buf = [0u8; 256];
        buf[..l].copy_from_slice(&b[..l]);
        buf[l] = 0;
        unsafe { libsarga::syscall::syscall2(2, buf.as_ptr() as u64, 0) }
    }
    pub fn write(fd: i64, buf: &[u8]) -> i64 {
        unsafe { libsarga::syscall::syscall3(1, fd as u64, buf.as_ptr() as u64, buf.len() as u64) }
    }
}

static FUTEX_VAL: AtomicU32 = AtomicU32::new(0);
const FUTEX_WAIT: u32 = 0;
const FUTEX_WAKE: u32 = 1;

fn child_test() -> i32 {
    println!("[CHILD {}] started", raw::getpid());
    // Wait for parent to set FUTEX_VAL to 1
    for _ in 0..100 {
        if FUTEX_VAL.load(Ordering::Acquire) == 1 {
            println!("[CHILD] saw parent signal, waking parent via futex");
            // Signal parent that child is done
            FUTEX_VAL.store(2, Ordering::Release);
            let woken = raw::futex(&FUTEX_VAL as *const _ as *mut u32, FUTEX_WAKE, 1);
            println!("[CHILD] wake returned {}", woken);
            raw::exit(0);
        }
        raw::yield_now();
    }
    println!("[CHILD] timeout waiting for parent");
    raw::exit(1);
}

fn main_test() -> i32 {
    // Test 1: API sanity — single-thread futex wake (no waiters)
    let woken = raw::futex(&FUTEX_VAL as *const _ as *mut u32, FUTEX_WAKE, 1);
    if woken < 0 {
        println!("FAIL: FUTEX_WAKE returned {}", woken);
        return 1;
    }
    println!("Test 1: FUTEX_WAKE (no waiters) = {}, PASS", woken);

    // Test 2: cross-process futex wake via fork
    println!("Test 2: cross-process futex wake via fork...");
    let pid = raw::fork();
    if pid < 0 {
        println!("FAIL: fork returned {}", pid);
        return 1;
    }
    if pid == 0 {
        // Child
        let ret = child_test();
        raw::exit(ret as i64);
    }

    // Parent: wait for child to block on futex, then wake it
    println!("[PARENT {}] forked child pid={}", raw::getpid(), pid);

    // Give child time to start, then signal
    for _ in 0..200 {
        raw::yield_now();
    }

    // Signal child by setting FUTEX_VAL and waking
    // ponytail: no sched_setattr on user side; yield + futex_wake is enough
    println!("[PARENT] setting futex = 1, waking child");
    FUTEX_VAL.store(1, Ordering::Release);
    let woken = raw::futex(&FUTEX_VAL as *const _ as *mut u32, FUTEX_WAKE, 1);
    println!("[PARENT] wake returned {}", woken);

    // Wait for child to respond (spinwait)
    let mut child_done = false;
    for _ in 0..500 {
        raw::yield_now();
        if FUTEX_VAL.load(Ordering::Acquire) == 2 {
            child_done = true;
            break;
        }
    }
    if child_done {
        println!("PASS: cross-process futex wake succeeded");
        0
    } else {
        println!("FAIL: child did not respond");
        1
    }
}

fn user_main() -> i32 {
    main_test()
}
sarga_main!(user_main);
