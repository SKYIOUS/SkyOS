use crate::syscall::syscall2;
use crate::syscall::syscall6;
use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

pub unsafe fn mmap(
    addr: u64,
    len: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: u64,
) -> Result<u64, i64> {
    let r = syscall6(
        9,
        addr,
        len as u64,
        prot as u64,
        flags as u64,
        fd as u64,
        offset,
    );
    if r < 0 {
        Err(-r)
    } else {
        Ok(r as u64)
    }
}

pub unsafe fn munmap(addr: u64, len: usize) -> Result<(), i64> {
    let r = syscall2(11, addr, len as u64);
    if r < 0 {
        Err(-r)
    } else {
        Ok(())
    }
}

pub fn brk(addr: u64) -> u64 {
    unsafe { crate::syscall::syscall1(12, addr) as u64 }
}

/// Slab allocator for small objects to reduce mmap overhead
const SLAB_SIZES: &[usize] = &[8, 16, 32, 64, 128, 256, 512, 1024, 2048];

struct SlabAllocator {
    free_lists: [AtomicUsize; SLAB_SIZES.len()], // Each stores a pointer to free list head
}

impl SlabAllocator {
    const fn new() -> Self {
        const ZERO: AtomicUsize = AtomicUsize::new(0);
        SlabAllocator {
            free_lists: [ZERO; SLAB_SIZES.len()],
        }
    }

    fn slab_index(&self, size: usize) -> Option<usize> {
        SLAB_SIZES.iter().position(|&s| s >= size)
    }

    unsafe fn alloc_from_slab(&self, layout: Layout) -> Option<*mut u8> {
        if let Some(idx) = self.slab_index(layout.size()) {
            let head = self.free_lists[idx].swap(0, Ordering::Acquire);
            if head != 0 {
                // Pop from free list
                let ptr = head as *mut u8;
                // Read next pointer from first word
                let next = *(ptr as *const usize);
                self.free_lists[idx].store(next, Ordering::Release);
                return Some(ptr);
            }
        }
        None
    }

    unsafe fn dealloc_to_slab(&self, ptr: *mut u8, layout: Layout) {
        if let Some(idx) = self.slab_index(layout.size()) {
            // Push to free list
            let head = self.free_lists[idx].load(Ordering::Acquire);
            *(ptr as *mut usize) = head;
            self.free_lists[idx].store(ptr as usize, Ordering::Release);
        }
    }
}

pub struct SargaMapper {
    slab: SlabAllocator,
}

impl SargaMapper {
    const fn new() -> Self {
        SargaMapper {
            slab: SlabAllocator::new(),
        }
    }
}

unsafe impl GlobalAlloc for SargaMapper {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Try slab allocator for small objects
        if let Some(ptr) = self.slab.alloc_from_slab(layout) {
            return ptr;
        }

        // Fall back to mmap for large allocations
        let size = (layout.size() + 4095) & !4095;
        match mmap(0, size, 3, 0x22, -1, 0) {
            // PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS
            Ok(ptr) => ptr as *mut u8,
            Err(_) => core::ptr::null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Try to return to slab allocator
        if self.slab.slab_index(layout.size()).is_some() {
            self.slab.dealloc_to_slab(ptr, layout);
            return;
        }

        // munmap for large allocations
        let size = (layout.size() + 4095) & !4095;
        let _ = munmap(ptr as u64, size);
    }
}

#[global_allocator]
pub static ALLOCATOR: SargaMapper = SargaMapper::new();

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    for i in 0..n {
        *dest.add(i) = *src.add(i);
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    for i in 0..n {
        *s.add(i) = c as u8;
    }
    s
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return (a as i32) - (b as i32);
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if dest < src as *mut u8 {
        for i in 0..n {
            *dest.add(i) = *src.add(i);
        }
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
    }
    dest
}
