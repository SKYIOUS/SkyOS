#![no_std]
#![no_main]
use libsarga::{sarga_main, print, println, io};

fn user_main() {
    // For a real `cat`, we'd parse argv. Since we don't have argv parsing yet in `_start`,
    // let's just make it read from stdin (fd 0) to stdout (fd 1).
    let mut buf = [0u8; 1024];
    loop {
        match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let _ = io::write(1, &buf[..n]);
            }
            Err(e) => {
                println!("cat error: {}", e);
                libsarga::process::exit(1);
            }
        }
    }
    
}

sarga_main!(user_main);
