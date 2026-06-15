use crate::syscall::*;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU32, Ordering};
use alloc::alloc::{alloc, Layout};

const FUTEX_WAIT: u32 = 0;
const FUTEX_WAKE: u32 = 1;

fn futex(uaddr: *mut u32, op: u32, val: u32) -> i64 {
    unsafe { syscall3(202, uaddr as u64, op as u64, val as u64) }
}

pub struct Thread {
    _tid: u32,
    clear_tid: Box<AtomicU32>,
}

pub fn spawn(f: fn()) -> Thread {
    let clear_tid = Box::new(AtomicU32::new(1));
    let clear_tid_ptr = &*clear_tid as *const AtomicU32 as *mut u32;

    let stack_size = 1024 * 1024;
    let stack_ptr = unsafe {
        let layout = Layout::from_size_align(stack_size, 4096).unwrap();
        alloc(layout)
    };
    let stack_top = stack_ptr as u64 + stack_size as u64;

    let func_ptr = f as usize;

    let flags = 0x100 | 0x80000 | 0x00200000 | 0x02000000;

    let tid = unsafe {
        let res = syscall6(
            56,
            flags,
            stack_top,
            0,
            func_ptr as u64,
            clear_tid_ptr as u64,
            0,
        );
        if res < 0 {
            panic!("thread::spawn failed: {}", res);
        }
        res as u32
    };

    Thread { _tid: tid, clear_tid }
}

impl Thread {
    pub fn join(self) {
        while self.clear_tid.load(Ordering::Acquire) != 0 {
            futex(self.clear_tid.as_ptr() as *mut u32, FUTEX_WAIT, 1);
        }
    }
}

pub struct Mutex {
    state: AtomicU32,
}

impl Mutex {
    pub const fn new() -> Self {
        Mutex { state: AtomicU32::new(0) }
    }

    pub fn lock(&self) {
        while self.state.swap(1, Ordering::Acquire) != 0 {
            futex(self.state_ptr(), FUTEX_WAIT, 1);
        }
    }

    pub fn unlock(&self) {
        self.state.store(0, Ordering::Release);
        futex(self.state_ptr(), FUTEX_WAKE, 1);
    }

    fn state_ptr(&self) -> *mut u32 {
        &self.state as *const AtomicU32 as *mut u32
    }
}
