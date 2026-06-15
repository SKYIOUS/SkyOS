#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::io;
use libsarga::fs;
use libsarga::args;

fn user_main() {
    if args::argc() < 2 {
        io::print_str("Usage: touch <file>...\n");
        return;
    }
    let mut i = 1;
    while i < args::argc() {
        let arg = args::get(i as usize).unwrap_or("");
        let r = fs::touch(arg);
        if r < 0 {
            io::print_str(&alloc::format!("touch: cannot create '{}'\n", arg));
        }
        i += 1;
    }
}

sarga_main!(user_main);
