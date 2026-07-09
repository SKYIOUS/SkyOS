//! Synchronization primitives.

use core::sync::atomic::{AtomicU32, Ordering};
use core::cell::UnsafeCell;
use crate::syscall::syscall3;

/// Low-level mutex based on futex.
pub struct RawMutex {
    state: AtomicU32, // 0 = unlocked, 1 = locked
}

impl RawMutex {
    /// Creates a new unlocked mutex.
    pub const fn new() -> Self {
        Self { state: AtomicU32::new(0) }
    }

    /// Acquires the lock, blocking the current thread if necessary.
    pub fn lock(&self) {
        loop {
            if self.state.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                break;
            }
            // FUTEX_WAIT: sleep while state == 1
            // SYS_FUTEX = 202
            unsafe { syscall3(202, self.state.as_ptr() as u64, 0, 1) };
        }
    }

    /// Releases the lock.
    pub fn unlock(&self) {
        self.state.store(0, Ordering::Release);
        // FUTEX_WAKE: wake one waiter
        unsafe { syscall3(202, self.state.as_ptr() as u64, 1, 1) };
    }
}

/// A mutual exclusion primitive for protecting shared data.
pub struct Mutex<T> {
    raw: RawMutex,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new mutex protecting the given value.
    pub const fn new(val: T) -> Self {
        Self {
            raw: RawMutex::new(),
            value: UnsafeCell::new(val),
        }
    }

    /// Acquires the lock and returns a guard for accessing the data.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.raw.lock();
        MutexGuard { mutex: self }
    }
}

/// An RAII guard for a `Mutex`.
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

/// Thread-Local Storage (TLS) key.
pub struct TlsKey {
    /// Offset within the TLS block.
    pub offset: usize,
}

impl TlsKey {
    /// Creates a new TLS key with the given offset.
    pub const fn new(offset: usize) -> Self {
        TlsKey { offset }
    }

    /// Retrieves the value stored in TLS for the current thread.
    pub fn get(&self) -> u64 {
        let mut base = 0u64;
        // SYS_ARCH_PRCTL = 158, ARCH_GET_FS = 0x1003
        unsafe { crate::syscall::syscall2(158, 0x1003, &mut base as *mut u64 as u64) };
        if base == 0 { return 0; }
        unsafe { *((base + self.offset as u64) as *const u64) }
    }

    /// Sets the value stored in TLS for the current thread.
    pub fn set(&self, val: u64) {
        let mut base = 0u64;
        // SYS_ARCH_PRCTL = 158, ARCH_GET_FS = 0x1003
        unsafe { crate::syscall::syscall2(158, 0x1003, &mut base as *mut u64 as u64) };
        if base == 0 { return; }
        unsafe { *((base + self.offset as u64) as *mut u64) = val; }
    }
}

/// Initializes Thread-Local Storage for the calling thread.
pub fn init_tls() -> u64 {
    // Allocate a 4KB page for TLS
    // SYS_MMAP = 9
    let addr = unsafe { crate::syscall::syscall2(9, 0, 4096) } as u64;
    if (addr as i64) < 0 || addr >= 0xFFFF_FFFF_FFFF_FF00 {
        return 0;
    }

    // Set FS base to the allocated TLS block
    // ARCH_SET_FS = 0x1002
    let ret = unsafe { crate::syscall::syscall2(158, 0x1002, addr) };
    if ret != 0 {
        return 0;
    }

    // Initialize errno at offset 0 to 0
    unsafe { *(addr as *mut i32) = 0; }

    addr
}
