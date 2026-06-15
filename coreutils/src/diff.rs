#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::io;
use libsarga::args;

fn read_file(path: &str) -> alloc::string::String {
    let fd = unsafe { libsarga::syscall::syscall2(2, path.as_ptr() as u64, 0) };
    if (fd as i64) < 0 { return alloc::string::String::new(); }
    let mut data = alloc::vec::Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libsarga::syscall::syscall3(0, fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (n as i64) <= 0 { break; }
        data.extend_from_slice(&buf[..n as usize]);
    }
    let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
    alloc::string::String::from_utf8_lossy(&data).into_owned()
}

fn user_main() {
    if args::argc() < 3 {
        io::print_str("Usage: diff <file1> <file2>\n");
        return;
    }
    let file1 = args::get(1).unwrap_or("");
    let file2 = args::get(2).unwrap_or("");
    let a = read_file(file1);
    let b = read_file(file2);
    let a_lines: alloc::vec::Vec<&str> = a.lines().collect();
    let b_lines: alloc::vec::Vec<&str> = b.lines().collect();
    let max = core::cmp::max(a_lines.len(), b_lines.len());
    let mut diffs = false;
    for i in 0..max {
        let al = a_lines.get(i).unwrap_or(&"");
        let bl = b_lines.get(i).unwrap_or(&"");
        if al != bl {
            diffs = true;
            io::print_str(&alloc::format!("{}: {} | {}\n", i + 1, al, bl));
        }
    }
    if !diffs {
        io::print_str("No differences\n");
    }
}

sarga_main!(user_main);
