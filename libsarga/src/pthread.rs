use crate::syscall;
use crate::thread;
use core::sync::atomic::{AtomicU32, Ordering};

pub type PthreadOnce = AtomicU32;
pub type PthreadKey = u32;

// ── Thread types ───────────────────────────────────────────────

#[repr(C)]
pub struct PthreadAttr {
    pub stack_size: usize,
    pub detach_state: PthreadDetachState,
    pub sched_policy: i32,
    pub sched_priority: i32,
}

#[repr(C)]
pub enum PthreadDetachState {
    Joinable = 0,
    Detached = 1,
}

impl PthreadAttr {
    pub fn new() -> Self {
        PthreadAttr {
            stack_size: 0,
            detach_state: PthreadDetachState::Joinable,
            sched_policy: 0,
            sched_priority: 0,
        }
    }
}

// ── Thread creation ────────────────────────────────────────────

pub type PthreadStartFn = extern "C" fn(*mut core::ffi::c_void) -> *mut core::ffi::c_void;

pub unsafe fn pthread_create(
    thread: &mut usize,
    _attr: Option<&PthreadAttr>,
    start: PthreadStartFn,
    arg: *mut core::ffi::c_void,
) -> i32 {
    let tid = unsafe { thread::spawn_raw(start, arg) };
    *thread = tid;
    0
}

pub fn pthread_join(thread: usize, _retval: *mut *mut core::ffi::c_void) -> i32 {
    thread::raw_thread_join(thread);
    0
}

pub fn pthread_detach(_thread: usize) -> i32 {
    0
}

pub fn pthread_self() -> usize {
    unsafe { syscall::sys_gettid() as usize }
}

pub fn pthread_exit(retval: *mut core::ffi::c_void) -> ! {
    thread::exit(retval as u64);
}

// ── Mutex (PTHREAD_MUTEX) ──────────────────────────────────────

#[repr(C)]
pub struct PthreadMutex {
    state: AtomicU32,
    kind: u32,
    owner: AtomicU32,
    lock_count: AtomicU32,
}

pub const PTHREAD_MUTEX_INITIALIZER: PthreadMutex = PthreadMutex {
    state: AtomicU32::new(0),
    kind: 0,
    owner: AtomicU32::new(0),
    lock_count: AtomicU32::new(0),
};

pub const PTHREAD_MUTEX_NORMAL: u32 = 0;
pub const PTHREAD_MUTEX_ERRORCHECK: u32 = 1;
pub const PTHREAD_MUTEX_RECURSIVE: u32 = 2;

pub fn pthread_mutex_init(_m: &PthreadMutex, _attr: Option<()>) -> i32 {
    0
}

pub fn pthread_mutex_lock(m: &PthreadMutex) -> i32 {
    if m.state
        .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        m.owner.store(pthread_self() as u32, Ordering::Relaxed);
        return 0;
    }
    loop {
        m.state.store(2, Ordering::Release);
        unsafe {
            syscall::sys_futex(
                &m.state as *const AtomicU32 as usize,
                syscall::FUTEX_WAIT,
                2,
                0,
                0,
                0,
            );
        }
        if m.state
            .compare_exchange(0, 2, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            m.owner.store(pthread_self() as u32, Ordering::Relaxed);
            return 0;
        }
    }
}

pub fn pthread_mutex_trylock(m: &PthreadMutex) -> i32 {
    if m.state
        .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
        .is_ok()
    {
        m.owner.store(pthread_self() as u32, Ordering::Relaxed);
        0
    } else {
        16 // EBUSY
    }
}

pub fn pthread_mutex_unlock(m: &PthreadMutex) -> i32 {
    let prev = m.state.swap(0, Ordering::Release);
    m.owner.store(0, Ordering::Relaxed);
    if prev == 2 {
        unsafe {
            syscall::sys_futex(
                &m.state as *const AtomicU32 as usize,
                syscall::FUTEX_WAKE,
                1,
                0,
                0,
                0,
            );
        }
    }
    0
}

pub fn pthread_mutex_destroy(_m: &PthreadMutex) -> i32 {
    0
}

// ── Condition variable ─────────────────────────────────────────

pub struct PthreadCond {
    state: AtomicU32,
}

pub const PTHREAD_COND_INITIALIZER: PthreadCond = PthreadCond {
    state: AtomicU32::new(0),
};

pub fn pthread_cond_wait(c: &PthreadCond, m: &PthreadMutex) -> i32 {
    c.state.store(1, Ordering::Release);
    pthread_mutex_unlock(m);
    while c.state.load(Ordering::Acquire) == 1 {
        unsafe {
            syscall::sys_futex(
                &c.state as *const AtomicU32 as usize,
                syscall::FUTEX_WAIT,
                1,
                0,
                0,
                0,
            );
        }
    }
    pthread_mutex_lock(m);
    0
}

pub fn pthread_cond_signal(c: &PthreadCond) -> i32 {
    c.state.store(0, Ordering::Release);
    unsafe {
        syscall::sys_futex(
            &c.state as *const AtomicU32 as usize,
            syscall::FUTEX_WAKE,
            1,
            0,
            0,
            0,
        );
    }
    0
}

pub fn pthread_cond_broadcast(c: &PthreadCond) -> i32 {
    c.state.store(0, Ordering::Release);
    unsafe {
        syscall::sys_futex(
            &c.state as *const AtomicU32 as usize,
            syscall::FUTEX_WAKE,
            i32::MAX as usize,
            0,
            0,
            0,
        );
    }
    0
}

pub fn pthread_cond_destroy(_c: &PthreadCond) -> i32 {
    0
}

// ── Once ───────────────────────────────────────────────────────

pub const PTHREAD_ONCE_INIT: PthreadOnce = AtomicU32::new(0);

pub fn pthread_once(once: &PthreadOnce, init: extern "C" fn()) {
    if once.load(Ordering::Acquire) != 1 {
        if once
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            init();
            once.store(2, Ordering::Release);
            unsafe {
                syscall::sys_futex(
                    once as *const AtomicU32 as usize,
                    syscall::FUTEX_WAKE,
                    usize::MAX,
                    0,
                    0,
                    0,
                );
            }
        } else {
            while once.load(Ordering::Acquire) == 1 {
                unsafe {
                    syscall::sys_futex(
                        once as *const AtomicU32 as usize,
                        syscall::FUTEX_WAIT,
                        1,
                        0,
                        0,
                        0,
                    );
                }
            }
        }
    }
}
