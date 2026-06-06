pub struct Mutex<T> {
    state: core::sync::atomic::AtomicU32,
    value: core::cell::UnsafeCell<T>,
}
unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(val: T) -> Self {
        Self { state: core::sync::atomic::AtomicU32::new(0),
               value: core::cell::UnsafeCell::new(val) }
    }
    pub fn lock(&self) -> MutexGuard<'_, T> {
        use core::sync::atomic::Ordering::*;
        loop {
            if self.state.compare_exchange(0, 1, Acquire, Relaxed).is_ok() { break; }
            // FUTEX_WAIT: sleep while state == 1
            unsafe { crate::syscall::syscall3(202,
                self.state.as_ptr() as u64, 0, 1) };
        }
        MutexGuard { mutex: self }
    }
    fn unlock(&self) {
        self.state.store(0, core::sync::atomic::Ordering::Release);
        // FUTEX_WAKE: wake one waiter
        unsafe { crate::syscall::syscall3(202,
            self.state.as_ptr() as u64, 1, 1) };
    }
}
pub struct MutexGuard<'a, T> { mutex: &'a Mutex<T> }
impl<T> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T { unsafe { &*self.mutex.value.get() } }
}
impl<T> core::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.mutex.value.get() } }
}
impl<T> Drop for MutexGuard<'_, T> { fn drop(&mut self) { self.mutex.unlock(); } }
