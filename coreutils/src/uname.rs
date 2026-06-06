#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, print, println, args, syscall::*};

fn field(buf: &[u8], start: usize) -> &str {
    core::str::from_utf8(&buf[start..start + 64]).unwrap_or("?").trim_end_matches('\0')
}

fn emit(first: &mut bool, s: &str) {
    if !*first { print!(" "); }
    print!("{}", s);
    *first = false;
}

fn user_main() {
    let mut buf = [0u8; 65 * 6];
    let r = unsafe { syscall1(63, buf.as_mut_ptr() as u64) };
    if r == 0 {
        let sysname = field(&buf, 0);
        let nodename = field(&buf, 65);
        let release = field(&buf, 130);
        let version = field(&buf, 195);
        let machine = field(&buf, 260);

        let mut print_all = false;
        let mut print_sysname = args::argc() == 1;
        let mut print_nodename = false;
        let mut print_release = false;
        let mut print_version = false;
        let mut print_machine = false;

        for i in 1..args::argc() {
            if let Some(arg) = args::get(i as usize) {
                match arg {
                    "-a" => {
                        print_all = true;
                    }
                    "-s" => print_sysname = true,
                    "-n" => print_nodename = true,
                    "-r" => print_release = true,
                    "-v" => print_version = true,
                    "-m" => print_machine = true,
                    _ => {}
                }
            }
        }

        if print_all {
            print_sysname = true;
            print_nodename = true;
            print_release = true;
            print_version = true;
            print_machine = true;
        }

        let mut first = true;

        if print_sysname { emit(&mut first, sysname); }
        if print_nodename { emit(&mut first, nodename); }
        if print_release { emit(&mut first, release); }
        if print_version { emit(&mut first, version); }
        if print_machine { emit(&mut first, machine); }
        if first {
            emit(&mut first, sysname);
        }
        println!();
    } else {
        println!("SkyOS");
    }
}

sarga_main!(user_main);
