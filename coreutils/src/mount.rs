#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args};
use libsarga::fs;

fn user_main() -> i32 {
    if args::argc() < 3 {
        println!("Usage: mount [-t fstype] <device> <target>");
        println!("       mount [-t fstype] <target>");
        println!("");
        println!("Filesystems: tmpfs, devfs, ctlfs, ext2");
        println!("If -t omitted, source is mounted with 'ext2' (block device)");
        println!("If source is 'none' or 'tmpfs', use 'tmpfs'");
        return 0;
    }

    let mut fstype = "ext2";
    let mut args_start = 1usize;
    if args_start < args::argc() as usize {
        if let Some("-t") = args::get(args_start) {
            fstype = args::get(args_start + 1).unwrap_or("ext2");
            args_start += 2;
        }
    }

    if args_start >= args::argc() as usize {
        println!("mount: missing operand");
        return 0;
    }

    let source = args::get(args_start).unwrap_or("none");
    if source == "none" || source == "tmpfs" {
        fstype = "tmpfs";
    }
    let target = args::get(args_start + 1).unwrap_or(source);

    match fs::mount(source, target, fstype, 0) {
        Ok(_) => {},
        Err(e) => println!("mount: {}: error {}", target, e),
    }
    0
}
sarga_main!(user_main);
