use core::alloc::{GlobalAlloc, Layout};
use crate::syscall::syscall6;
use crate::syscall::syscall2;

pub unsafe fn mmap(addr: u64, len: usize, prot: i32, flags: i32, fd: i32, offset: u64) -> Result<u64, i64> {
    let r = syscall6(9, addr, len as u64, prot as u64, flags as u64, fd as u64, offset);
    if r < 0 { Err(-r) } else { Ok(r as u64) }
}

pub unsafe fn munmap(addr: u64, len: usize) -> Result<(), i64> {
    let r = syscall2(11, addr, len as u64);
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn brk(addr: u64) -> u64 {
    unsafe { crate::syscall::syscall1(12, addr) as u64 }
}

pub struct SargaMapper;

unsafe impl GlobalAlloc for SargaMapper {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = (layout.size() + 4095) & !4095;
        match mmap(0, size, 3, 0x22, -1, 0) { // PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS
            Ok(ptr) => ptr as *mut u8,
            Err(_) => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = (layout.size() + 4095) & !4095;
        let _ = munmap(ptr as u64, size);
    }
}

#[global_allocator]
pub static ALLOCATOR: SargaMapper = SargaMapper;

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
