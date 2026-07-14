use crate::syscall::*;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU32, Ordering};
use alloc::alloc::{alloc, Layout};
use alloc::vec::Vec;
use crate::sync::Mutex as SargaMutex;

fn futex(uaddr: *mut u32, op: u32, val: u32) -> i64 {
    unsafe { syscall3(SYS_FUTEX, uaddr as u64, op as u64, val as u64) }
}

// ponytail: Vec-based thread table, fine for typical workloads
struct RawThreadTable(Vec<(usize, *mut u32)>);
unsafe impl Send for RawThreadTable {}
unsafe impl Sync for RawThreadTable {}

static RAW_THREADS: SargaMutex<RawThreadTable> = SargaMutex::new(RawThreadTable(Vec::new()));

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
            futex(self.clear_tid.as_ptr() as *mut u32, crate::syscall::FUTEX_WAIT, 1);
        }
    }
}

pub unsafe fn spawn_raw(
    start: extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void,
    arg: *mut core::ffi::c_void,
) -> usize {
    let clear_tid = Box::new(AtomicU32::new(1));
    let clear_tid_ptr = &*clear_tid as *const AtomicU32 as *mut u32;

    let stack_size = 1024 * 1024;
    let stack_ptr = {
        let layout = Layout::from_size_align(stack_size, 4096).unwrap();
        alloc(layout)
    };
    let stack_top = stack_ptr as u64 + stack_size as u64;

    let sp = stack_top as *mut u64;
    sp.offset(-1).write(start as u64);
    sp.offset(-2).write(arg as u64);
    let child_rsp = (sp as u64).wrapping_sub(16);

    extern "C" fn trampoline() {
        unsafe {
            core::arch::asm!(
                "pop rdi",
                "pop rax",
                "call rax",
                "mov rdi, rax",
                "mov rax, 60",
                "syscall",
                options(noreturn)
            );
        }
    }

    let flags = 0x100 | 0x80000 | 0x00200000 | 0x02000000;
    let tid = crate::syscall::syscall6(
        crate::syscall::SYS_CLONE,
        flags,
        child_rsp,
        0,
        trampoline as *const () as u64,
        clear_tid_ptr as u64,
        0,
    );
    if tid < 0 {
        panic!("spawn_raw failed: {}", tid);
    }
    let _ = Box::into_raw(clear_tid);
    RAW_THREADS.lock().0.push((tid as usize, clear_tid_ptr));
    tid as usize
}

pub fn raw_thread_join(tid: usize) {
    loop {
        let addr = {
            let guard = RAW_THREADS.lock();
            guard.0.iter().find(|e| e.0 == tid).map(|e| e.1)
        };
        match addr {
            Some(addr) if unsafe { *addr != 0 } => {
                futex(addr, crate::syscall::FUTEX_WAIT, 1);
            }
            _ => break,
        }
    }
    let mut guard = RAW_THREADS.lock();
    guard.0.retain(|e| e.0 != tid);
}

pub fn exit(code: u64) -> ! {
    unsafe { crate::syscall::syscall1(SYS_EXIT, code) };
    loop {}
}

pub fn sleep_ms(ms: u64) {
    unsafe { crate::syscall::syscall2(SYS_NANOSLEEP, ms * 1_000_000, 0) };
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
            futex(self.state_ptr(), crate::syscall::FUTEX_WAIT, 1);
        }
    }

    pub fn unlock(&self) {
        self.state.store(0, Ordering::Release);
        futex(self.state_ptr(), crate::syscall::FUTEX_WAKE, 1);
    }

    fn state_ptr(&self) -> *mut u32 {
        &self.state as *const AtomicU32 as *mut u32
    }
}
