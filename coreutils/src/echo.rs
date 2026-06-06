#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, print, println, args};

fn user_main() {
    let mut first = true;
    let mut newline = true;
    let mut start = 1usize;

    if args::argc() > 1 {
        if let Some(flag) = args::get(1) {
            if flag == "-n" {
                newline = false;
                start = 2;
            }
        }
    }

    for i in start..(args::argc() as usize) {
        if let Some(s) = args::get(i as usize) {
            if !first { print!(" "); }
            print!("{}", s);
            first = false;
        }
    }
    if newline {
        println!();
    }
}
sarga_main!(user_main);
