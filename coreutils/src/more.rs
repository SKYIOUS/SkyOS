#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::ToString;
use libsarga::{args, io, println, sarga_main};

fn user_main() -> i32 {
    let lines: alloc::vec::Vec<alloc::string::String> = if args::argc() > 1 {
        let path = args::get(1).unwrap();
        match io::read_to_string(path) {
            Ok(s) => s.lines().map(|l| l.to_string()).collect(),
            Err(_) => {
                println!("more: {}: No such file", path);
                return 1;
            }
        }
    } else {
        let mut buf = [0u8; 4096];
        let mut all = alloc::string::String::new();
        loop {
            match io::read(0, &mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                        all.push_str(s);
                    }
                }
                Err(_) => break,
            }
        }
        all.lines().map(|l| l.to_string()).collect()
    };

    let mut i = 0;
    while i < lines.len() {
        let end = core::cmp::min(i + 24, lines.len());
        for line in lines.iter().take(end).skip(i) {
            println!("{}", line);
        }
        i = end;
        if i < lines.len() {
            println!("--More--({}%)", i * 100 / lines.len());
            let mut k = [0u8; 1];
            let _ = io::read(0, &mut k);
        }
    }
    0
}
sarga_main!(user_main);
