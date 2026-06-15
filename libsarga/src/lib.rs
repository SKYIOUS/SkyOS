#![no_std]
#![feature(alloc_error_handler)]

pub extern crate alloc;

pub mod mem;
pub mod syscall;
pub mod io;
pub mod process;
pub mod gui;
pub mod ai;
pub mod vahiai;
pub mod sync;
pub mod start;
pub mod errno;
pub mod stdio;
pub mod args;
pub mod net;
pub mod gpu;
pub mod hash;
pub mod fs;
pub mod thread;
pub mod posix;

// Widget toolkit
pub mod theme;
pub mod png;
pub mod widget;
pub mod button;
pub mod label;
pub mod textbox;
pub mod progress_bar;
pub mod checkbox;
pub mod tab_widget;
pub mod menubar;
pub mod dialog;
pub mod combobox;
pub mod slider;
pub mod scrollbar;
pub mod layout;

#[macro_export]
macro_rules! sarga_main {
    ($main_fn:path) => {
        #[no_mangle]
        pub extern "Rust" fn main() -> i32 {
            $main_fn();
            0
        }
    };
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    crate::println!("SARGA OS PANIC: {}", info);
    process::exit(1);
}
