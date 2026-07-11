use core::sync::atomic::{AtomicU32, Ordering};
use core::cell::UnsafeCell;
use crate::syscall::syscall3;

// Raw Mutex (no value wrapping)
pub struct RawMutex {
    state: AtomicU32, // 0 = unlocked, 1 = locked
}

impl RawMutex {
    pub const fn new() -> Self {
        Self { state: AtomicU32::new(0) }
    }

    pub fn lock(&self) {
        loop {
            if self.state.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                break;
            }
            // FUTEX_WAIT: sleep while state == 1
            unsafe { syscall3(202, self.state.as_ptr() as u64, 0, 1) };
        }
    }

    pub fn unlock(&self) {
        self.state.store(0, Ordering::Release);
        // FUTEX_WAKE: wake one waiter
        unsafe { syscall3(202, self.state.as_ptr() as u64, 1, 1) };
    }
}

// Safe Mutex wrapping a value
pub struct Mutex<T> {
    raw: RawMutex,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
// SAFETY: Mutex provides mutual exclusion; T: Send allows moving between threads,
// T: Sync is needed because lock() grants shared &T access via Deref on the guard.
unsafe impl<T: Send + Sync> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(val: T) -> Self {
        Self {
            raw: RawMutex::new(),
            value: UnsafeCell::new(val),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.raw.lock();
        MutexGuard { mutex: self }
    }
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<T> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> core::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.mutex.raw.unlock();
    }
}

// TLS (Thread-Local Storage)
pub struct TlsKey {
    pub offset: usize,
}

impl TlsKey {
    pub const fn new(offset: usize) -> Self {
        TlsKey { offset }
    }

    pub fn get(&self) -> u64 {
        let mut base = 0u64;
        unsafe { crate::syscall::syscall2(158, 0x1003, &mut base as *mut u64 as u64) }; // ARCH_GET_FS
        if base == 0 { return 0; }
        unsafe { *((base + self.offset as u64) as *const u64) }
    }

    pub fn set(&self, val: u64) {
        let mut base = 0u64;
        unsafe { crate::syscall::syscall2(158, 0x1003, &mut base as *mut u64 as u64) }; // ARCH_GET_FS
        if base == 0 { return; }
        unsafe { *((base + self.offset as u64) as *mut u64) = val; }
    }
}

pub fn init_tls() -> u64 {
    // Allocate a 4KB page for TLS
    let addr = unsafe { crate::syscall::syscall2(9, 0, 4096) } as u64;
    if (addr as i64) < 0 || addr >= 0xFFFF_FFFF_FFFF_FF00 {
        return 0;
    }

    // Set FS base to the allocated TLS block
    let ret = unsafe { crate::syscall::syscall2(158, 0x1002, addr) }; // ARCH_SET_FS
    if ret != 0 {
        return 0;
    }

    // Initialize errno at offset 0 to 0
    unsafe { *(addr as *mut i32) = 0; }

    addr
}
