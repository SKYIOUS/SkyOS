#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, args, io, println};
use alloc::string::{String, ToString};

fn user_main() -> i32 {
    if args::argc() < 2 { return 0; }
    let fmt = args::get(1).unwrap_or("");
    if fmt.is_empty() { return 0; }
    let mut arg_idx = 2usize;
    let mut out = String::new();
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            out.push(match chars.next() {
                Some('n') => '\n',
                Some('t') => '\t',
                Some('\\') => '\\',
                Some(c) => c,
                None => break,
            });
        } else if c == '%' {
            match chars.next() {
                Some('s') => { out.push_str(args::get(arg_idx).unwrap_or("")); arg_idx += 1; }
                Some('d') => {
                    let s = args::get(arg_idx).unwrap_or("0");
                    let n: i64 = s.parse().unwrap_or(0);
                    out.push_str(&n.to_string());
                    arg_idx += 1;
                }
                Some('x') | Some('X') => {
                    let s = args::get(arg_idx).unwrap_or("0");
                    let n: u64 = s.parse().unwrap_or(0);
                    out.push_str(&alloc::format!("{:x}", n));
                    arg_idx += 1;
                }
                Some('%') => out.push('%'),
                Some(c) => { out.push('%'); out.push(c); }
                None => out.push('%'),
            }
        } else {
            out.push(c);
        }
    }
    io::print_str(&out);
    0
}

sarga_main!(user_main);
