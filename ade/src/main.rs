#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, gui::Window, io};
use alloc::vec::Vec;
use alloc::string::String;

fn user_main() {
    println!("Aethos Desktop Environment (ade) starting...");
    let mut desktop = match Window::create("ADE Root Compositor", 1024, 768) {
        Ok(w) => w,
        Err(e) => {
            println!("ADE failed to create root window: {}", e);
            return;
        }
    };
    desktop.fill(0x00_1E_3A_8A);
    desktop.draw_rect(0, 768 - 40, 1024, 40, 0x00_1F_29_37);
    if let Err(_) = desktop.flush() {
        println!("ADE failed to flush framebuffer");
    }
    println!("ADE running. IPC listening on /dev/ade/request (placeholder).");
    loop {
        unsafe { libsarga::syscall::syscall1(35, [0u64, 100_000_000u64].as_ptr() as u64) };
    }
}

sarga_main!(user_main);
