#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, print, sarga_main};

fn user_main() -> i32 {
    let newline = !args::get(1).map_or(false, |a| a == "-n");
    let start = if !newline && args::argc() > 1 { 2 } else { 1 };

    for i in start..args::argc() {
        if let Some(a) = args::get(i as usize) {
            if i > start {
                print!(" ");
            }
            print!("{}", a);
        }
    }
    if newline {
        print!("\n");
    }
    0
}
sarga_main!(user_main);
