#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::io;
use libsarga::println;

fn user_main() -> i32 {
    println!("SARGAAI v0.3 — Type 'help' for commands.");
    loop {
        io::write_all(1, b"ai> ").ok();
        let mut input = [0u8; 512];
        let n = io::read(0, &mut input).unwrap_or(0);
        if n == 0 { break; }
        let s = core::str::from_utf8(&input[..n]).unwrap_or("").trim();
        if s.is_empty() { continue; }
        match libsarga::vahiai::handle_intent(s) {
            Ok(resp) => println!("{}", resp),
            Err(e) => println!("Error: {}", e),
        }
    }
    0
}
sarga_main!(user_main);
