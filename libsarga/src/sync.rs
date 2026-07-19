use core::sync::atomic::{AtomicU32, Ordering};
use core::cell::UnsafeCell;
use crate::syscall::syscall3;
use crate::syscall::SYS_FUTEX;

// Raw Mutex (no value wrapping)
pub struct RawMutex {
    state: AtomicU32, // 0 = unlocked, 1 = locked
}

impl RawMutex {
    pub const fn new() -> Self {
        Self { state: AtomicU32::new(0) }
    }

    pub fn lock(&self) {
        self.try_lock_timeout(None).expect("Mutex lock failed");
    }

    /// Attempt to acquire the lock with a timeout in milliseconds.
    /// Returns Ok(()) if lock acquired, Err(()) if timeout elapsed.
    pub fn try_lock_timeout(&self, timeout_ms: Option<u64>) -> Result<(), ()> {
        let start = if timeout_ms.is_some() {
            Some(unsafe { crate::syscall::syscall1(96, 0) }) // clock_gettime
        } else {
            None
        };

        loop {
            if self.state.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                return Ok(());
            }

            if let Some(timeout) = timeout_ms {
                let now = unsafe { crate::syscall::syscall1(96, 0) };
                if now - start.unwrap() > (timeout * 1_000_000) as i64 {
                    return Err(());
                }
            }

            // SAFETY: FUTEX_WAIT syscall is safe here - state.as_ptr() is a valid pointer to AtomicU32,
            // and we're passing valid futex operation codes. The syscall will block until woken.
            unsafe { syscall3(202, self.state.as_ptr() as u64, 0, 1) };
        }
    }

    pub fn unlock(&self) {
        self.state.store(0, Ordering::Release);
        // SAFETY: FUTEX_WAKE syscall is safe here - state.as_ptr() is a valid pointer to AtomicU32,
        // and waking waiters is safe even if none are waiting.
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

// ── Read-Write Lock (RwLock) ───────────────────────────────────────

pub struct RwLock {
    state: AtomicU32, // Bit 31: writer lock, bits 0-30: reader count
}

const RWLOCK_WRITER: u32 = 1 << 31;
const RWLOCK_READER_MASK: u32 = (1 << 31) - 1;

impl RwLock {
    pub const fn new() -> Self {
        Self { state: AtomicU32::new(0) }
    }

    pub fn read(&self) {
        loop {
            let state = self.state.load(Ordering::Acquire);
            // Check if writer is not holding lock and reader count won't overflow
            if (state & RWLOCK_WRITER) == 0 && (state & RWLOCK_READER_MASK) < RWLOCK_READER_MASK {
                if self.state.compare_exchange(state, state + 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                    return;
                }
            }
            // SAFETY: FUTEX_WAIT syscall is safe here - state.as_ptr() is a valid pointer to AtomicU32
            unsafe { syscall3(202, self.state.as_ptr() as u64, 0, 1) };
        }
    }

    pub fn try_read(&self) -> bool {
        let state = self.state.load(Ordering::Acquire);
        if (state & RWLOCK_WRITER) == 0 && (state & RWLOCK_READER_MASK) < RWLOCK_READER_MASK {
            self.state.compare_exchange(state, state + 1, Ordering::Acquire, Ordering::Relaxed).is_ok()
        } else {
            false
        }
    }

    pub fn write(&self) {
        loop {
            let state = self.state.load(Ordering::Acquire);
            // Try to acquire writer lock if no readers or writer
            if state == 0 {
                if self.state.compare_exchange(0, RWLOCK_WRITER, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                    return;
                }
            }
            // SAFETY: FUTEX_WAIT syscall is safe here - state.as_ptr() is a valid pointer to AtomicU32
            unsafe { syscall3(202, self.state.as_ptr() as u64, 0, 1) };
        }
    }

    pub fn try_write(&self) -> bool {
        self.state.compare_exchange(0, RWLOCK_WRITER, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }

    pub fn read_unlock(&self) {
        let prev = self.state.fetch_sub(1, Ordering::Release);
        // If we were the last reader, wake any waiting writer
        if prev == 1 {
            unsafe { syscall3(202, self.state.as_ptr() as u64, 1, 1) };
        }
    }

    pub fn write_unlock(&self) {
        self.state.store(0, Ordering::Release);
        // Wake all waiting readers and writers
        unsafe { syscall3(202, self.state.as_ptr() as u64, 1, i32::MAX as u64) };
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

// ── Condition Variable ──────────────────────────────────────────

pub struct Condvar {
    state: AtomicU32,
}

impl Condvar {
    pub const fn new() -> Self {
        Self { state: AtomicU32::new(0) }
    }

    pub fn wait(&self, mutex: &RawMutex) {
        self.wait_timeout(mutex, None);
    }

    /// Wait with timeout in milliseconds. Returns true if signaled, false if timeout.
    pub fn wait_timeout(&self, mutex: &RawMutex, timeout_ms: Option<u64>) -> bool {
        let start = if timeout_ms.is_some() {
            Some(unsafe { crate::syscall::syscall1(96, 0) }) // clock_gettime
        } else {
            None
        };

        self.state.store(1, Ordering::Release);
        mutex.unlock();

        loop {
            // Check for spurious wakeups - only return if state is 0 (signaled)
            if self.state.load(Ordering::Acquire) == 0 {
                mutex.lock();
                return true;
            }

            if let Some(timeout) = timeout_ms {
                let now = unsafe { crate::syscall::syscall1(96, 0) };
                if now - start.unwrap() > (timeout * 1_000_000) as i64 {
                    mutex.lock();
                    return false;
                }
            }

            // SAFETY: FUTEX_WAIT syscall is safe here - state.as_ptr() is a valid pointer to AtomicU32
            unsafe { syscall3(SYS_FUTEX, self.state.as_ptr() as u64, 0, 1) };
        }
    }

    pub fn signal(&self) {
        self.state.store(0, Ordering::Release);
        unsafe { syscall3(SYS_FUTEX, self.state.as_ptr() as u64, 1, 1) };
    }

    pub fn broadcast(&self) {
        self.state.store(0, Ordering::Release);
        unsafe { syscall3(SYS_FUTEX, self.state.as_ptr() as u64, 1, i32::MAX as u64) };
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
