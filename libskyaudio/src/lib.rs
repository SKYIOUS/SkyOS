#![no_std]

extern crate alloc;
extern crate libsarga;

pub fn beep(freq_hz: u32, duration_ms: u32) {
    unsafe { libsarga::syscall::beep(freq_hz, duration_ms); }
}

pub fn play_wav(data: &[u8]) {
    // Basic implementation: write to /dev/speaker
    let fd = unsafe { libsarga::syscall::open("/dev/speaker\0".as_ptr() as *const u8, 1) }; // O_WRONLY
    if fd >= 0 {
        unsafe { libsarga::syscall::write(fd, data.as_ptr(), data.len()); }
        unsafe { libsarga::syscall::close(fd); }
    }
}
