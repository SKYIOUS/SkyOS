#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, io, println, sarga_main};

fn user_main() -> i32 {
    match io::read_to_string("/etc/passwd") {
        Ok(s) => {
            for line in s.lines() {
                if let Some(name) = line.split(':').next() {
                    print!("{} ", name);
                }
            }
            println!("");
            0
        }
        Err(_) => { println!("(unknown)"); 1 }
    }
}
sarga_main!(user_main);