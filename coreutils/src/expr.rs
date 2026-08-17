#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, println, sarga_main};

fn apply_op(a: i64, op: &str, b: i64) -> Option<i64> {
    match op {
        "+" => Some(a + b),
        "-" => Some(a - b),
        "*" => Some(a * b),
        "/" => Some(if b != 0 {
            a / b
        } else {
            return None;
        }),
        "%" => Some(if b != 0 {
            a % b
        } else {
            return None;
        }),
        "=" => Some(if a == b { 1 } else { 0 }),
        "!=" => Some(if a != b { 1 } else { 0 }),
        "<" => Some(if a < b { 1 } else { 0 }),
        "<=" => Some(if a <= b { 1 } else { 0 }),
        ">" => Some(if a > b { 1 } else { 0 }),
        ">=" => Some(if a >= b { 1 } else { 0 }),
        _ => None,
    }
}

fn user_main() -> i32 {
    if args::argc() < 4 {
        println!("usage: expr a op b");
        return 1;
    }
    let a: i64 = match args::get(1).unwrap_or("0").parse() {
        Ok(n) => n,
        Err(_) => {
            println!("expr: non-integer");
            return 1;
        }
    };
    let op = args::get(2).unwrap_or("");
    let b: i64 = match args::get(3).unwrap_or("0").parse() {
        Ok(n) => n,
        Err(_) => {
            println!("expr: non-integer");
            return 1;
        }
    };
    match apply_op(a, op, b) {
        Some(r) => {
            println!("{}", r);
            0
        }
        None => {
            println!("expr: error");
            1
        }
    }
}
sarga_main!(user_main);
