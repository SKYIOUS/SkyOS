#[no_mangle]
#[link_section = ".text._start"]
pub unsafe extern "C" fn _start() -> ! {
    extern "Rust" { fn main(); }
    main();
    crate::process::exit(0);
}
