#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, println, sarga_main};

fn user_main() -> i32 {
    let argc = args::argc();
    let (first, incr, last) = match argc {
        2 => (
            1.0,
            1.0,
            args::get(1).unwrap_or("1").parse::<f64>().unwrap_or(1.0),
        ),
        3 => (
            args::get(1).unwrap_or("1").parse::<f64>().unwrap_or(1.0),
            1.0,
            args::get(2).unwrap_or("1").parse::<f64>().unwrap_or(1.0),
        ),
        4 => (
            args::get(1).unwrap_or("1").parse::<f64>().unwrap_or(1.0),
            args::get(2).unwrap_or("1").parse::<f64>().unwrap_or(1.0),
            args::get(3).unwrap_or("1").parse::<f64>().unwrap_or(1.0),
        ),
        _ => {
            println!("usage: seq [first [incr]] last");
            return 1;
        }
    };
    if incr == 0.0 {
        return 1;
    }
    let mut i = first;
    while if incr > 0.0 { i <= last } else { i >= last } {
        if i == (i as i64) as f64 {
            println!("{}", i as i64);
        } else {
            println!("{}", i);
        }
        i += incr;
    }
    0
}
sarga_main!(user_main);
