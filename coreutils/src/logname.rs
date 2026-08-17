#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{io, println, sarga_main};

fn user_main() -> i32 {
    // Try /etc/passwd first, fallback to getlogin-like behavior
    match io::read_to_string("/etc/passwd") {
        Ok(passwd) => {
            for line in passwd.lines() {
                let name = match line.split(':').next() {
                    Some(n) => n,
                    None => continue,
                };
                println!("{}", name);
                return 0;
            }
            println!("(unknown)");
        }
        Err(_) => println!("(unknown)"),
    }
    1
}
sarga_main!(user_main);
