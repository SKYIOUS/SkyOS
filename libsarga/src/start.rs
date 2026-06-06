#[no_mangle]
#[link_section = ".text._start"]
pub unsafe extern "C" fn _start() -> ! {
    extern "Rust" { fn main() -> i32; }
    let code = main();
    crate::process::exit(code);
}
