#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, ai, io};
use alloc::string::String;

fn main() -> i32 {
    println!("Aethos SkyAI CLI");
    println!("Type 'exit' to quit.");
    
    let mut buf = [0u8; 1024];
    loop {
        libsky::print!("ai> ");
        match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let input = core::str::from_utf8(&buf[..n]).unwrap_or("").trim();
                if input == "exit" {
                    break;
                }
                if input.is_empty() { continue; }
                
                match ai::query(input) {
                    Ok(resp) => println!("{}", resp),
                    Err(e) => println!("AI Error: {}", e),
                }
            }
            Err(_) => break,
        }
    }
    0
}

sarga_main!(main);
