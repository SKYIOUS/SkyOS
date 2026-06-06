#![no_std]
#![feature(alloc_error_handler)]

pub extern crate alloc;

pub mod mem;
pub mod syscall;
pub mod io;
pub mod process;
pub mod gui;
pub mod ai;
pub mod sync;
pub mod start;
pub mod errno;
pub mod stdio;
pub mod args;
pub mod net;

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
