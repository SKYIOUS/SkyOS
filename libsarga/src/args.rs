use core::sync::atomic::{AtomicI32, Ordering};

static ARGC: AtomicI32 = AtomicI32::new(0);
static ARGV: AtomicI32 = AtomicI32::new(0);

pub fn init(stack: *const u64) {
    let argc = unsafe { *stack } as i32;
    ARGC.store(argc, Ordering::SeqCst);
    ARGV.store(stack as i32 + 8, Ordering::SeqCst);
}

pub fn argc() -> i32 {
    ARGC.load(Ordering::Relaxed)
}

pub fn argv() -> *const *const u8 {
    ARGV.load(Ordering::Relaxed) as *const *const u8
}

pub fn get(pos: usize) -> Option<&'static str> {
    let argv = argv();
    if pos >= argc() as usize { return None; }
    unsafe {
        let ptr = *argv.add(pos);
        if ptr.is_null() { return None; }
        let mut len = 0;
        while *ptr.add(len) != 0 { len += 1; }
        core::str::from_utf8(core::slice::from_raw_parts(ptr, len)).ok()
    }
}
