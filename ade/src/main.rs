#![no_std]
#![no_main]
extern crate alloc;
use libsky::{aethos_main, println, gui::Window, io, syscall::*};
use alloc::vec::Vec;
use alloc::string::String;

fn main() -> i32 {
    println!("Aethos Desktop Environment (ade) starting...");
    
    // ADE acquires the full screen framebuffer (assume 1024x768 for safe default)
    let mut desktop = match Window::create("ADE Root Compositor", 1024, 768) {
        Ok(w) => w,
        Err(e) => {
            println!("ADE failed to create root window: {}", e);
            return 1;
        }
    };
    
    // Draw background (Aethos Blue)
    desktop.fill(0x00_1E_3A_8A); 
    
    // Draw taskbar (Dark Gray)
    desktop.draw_rect(0, 768 - 40, 1024, 40, 0x00_1F_29_37);
    
    if let Err(_) = desktop.flush() {
        println!("ADE failed to flush framebuffer");
    }

    println!("ADE running. IPC listening on /dev/ade/request (placeholder).");
    
    loop {
        // Here ADE would check IPC messages from child apps (Terminal, Files, etc.)
        // For now, idle loop to keep compositor alive.
        unsafe { syscall1(SYS_NANOSLEEP, [0u64, 100_000_000u64].as_ptr() as u64) };
    }
}

aethos_main!(main);
