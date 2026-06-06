use crate::syscall::*;
use alloc::format;

const SYS_GUI_CREATE_WINDOW: u64 = 100;
const SYS_GUI_GET_BUFFER: u64 = 101;
const SYS_GUI_FLUSH: u64 = 102;

pub struct Window {
    id: u64,
    width: u32,
    height: u32,
    buffer: &'static mut [u32],  // Mapped from gui_get_buffer syscall
}

impl Window {
    pub fn create(title: &str, width: u32, height: u32) -> Result<Self, i64> {
        let title_c = format!("{}\0", title);
        let id = unsafe { syscall3(SYS_GUI_CREATE_WINDOW,
            title_c.as_ptr() as u64, width as u64, height as u64) };
        if id < 0 { return Err(-id); }

        // Map the shared framebuffer
        let buf_ptr = unsafe { syscall2(SYS_GUI_GET_BUFFER, id as u64, 0) } as *mut u32;
        let buffer = unsafe {
            core::slice::from_raw_parts_mut(buf_ptr, (width * height) as usize)
        };
        Ok(Window { id: id as u64, width, height, buffer })
    }

    pub fn buffer_mut(&mut self) -> &mut [u32] { self.buffer }

    pub fn flush(&self) -> Result<(), i64> {
        let ret = unsafe { syscall1(SYS_GUI_FLUSH, self.id) };
        if ret < 0 { Err(-ret) } else { Ok(()) }
    }

    pub fn fill(&mut self, color: u32) {
        for px in self.buffer.iter_mut() { *px = color; }
    }

    pub fn draw_rect(&mut self, x: u32, y: u32, w: u32, h: u32, color: u32) {
        for dy in 0..h {
            for dx in 0..w {
                let px = (x + dx) as usize;
                let py = (y + dy) as usize;
                if px < self.width as usize && py < self.height as usize {
                    self.buffer[py * self.width as usize + px] = color;
                }
            }
        }
    }
}
