#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::io;
use libsarga::args;

fn read_file(path: &str) -> alloc::vec::Vec<u8> {
    let fd = unsafe { libsarga::syscall::syscall2(2, path.as_ptr() as u64, 0) };
    if (fd as i64) < 0 { return alloc::vec::Vec::new(); }
    let mut data = alloc::vec::Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libsarga::syscall::syscall3(0, fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (n as i64) <= 0 { break; }
        data.extend_from_slice(&buf[..n as usize]);
    }
    let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
    data
}

fn oct_to_num(s: &str) -> u64 {
    let mut n: u64 = 0;
    for b in s.bytes() {
        if b >= b'0' && b <= b'7' {
            n = n * 8 + (b - b'0') as u64;
        }
    }
    n
}

fn user_main() -> i32 {
    let mut extract = false;
    let mut list = false;
    let mut create = false;
    let mut file = "";
    let mut i = 1;
    while i < args::argc() {
        let arg = args::get(i as usize).unwrap_or("");
        if arg == "-x" || arg == "--extract" { extract = true; }
        else if arg == "-t" || arg == "--list" { list = true; }
        else if arg == "-c" || arg == "--create" { create = true; }
        else if arg.starts_with("-f") {
            file = if arg.len() > 2 { &arg[2..] } else {
                i += 1;
                args::get(i as usize).unwrap_or("")
            };
        } else if !arg.starts_with('-') && file.is_empty() {
            file = arg;
        }
        i += 1;
    }
    if file.is_empty() {
        io::print_str("Usage: tar -xtcf <archive> [files]\n");
        return 0;
    }
    if create {
        io::print_str("tar: create not yet supported\n");
        return 0;
    }
    let data = read_file(file);
    if data.is_empty() {
        io::print_str(&alloc::format!("tar: cannot open '{}'\n", file));
        return 0;
    }
    let mut offset = 0;
    while offset + 512 <= data.len() {
        let header = &data[offset..offset + 512];
        if header.iter().all(|&b| b == 0) { break; }
        let name_raw = &header[0..100];
        let name_end = name_raw.iter().position(|&b| b == 0).unwrap_or(100);
        let name = alloc::string::String::from_utf8_lossy(&name_raw[..name_end]);
        let size_field = alloc::string::String::from_utf8_lossy(&header[124..136]);
        let size = oct_to_num(size_field.trim());
        let typeflag = if header[156] == 0 { '0' } else { header[156] as char };
        if list || (extract && !name.is_empty()) {
            if list {
                let mode = oct_to_num(alloc::string::String::from_utf8_lossy(&header[100..108]).trim());
                io::print_str(&alloc::format!(
                    "{:>6o} {:>10} {}\n", mode & 0o7777, size, name
                ));
            } else {
                io::print_str(&alloc::format!("{}/{}\n", file, name));
            }
        }
        if extract && typeflag != '5' && typeflag != 'd' && !name.is_empty() {
            let file_data = &data[offset + 512..offset + 512 + size as usize];
            let flags = 0x241u64;
            let fd = unsafe { libsarga::syscall::syscall2(2, name.as_ptr() as u64, flags) };
            if (fd as i64) >= 0 {
                let _ = unsafe { libsarga::syscall::syscall3(1, fd as u64, file_data.as_ptr() as u64, size) };
                let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
                io::print_str(&alloc::format!("x {}\n", name));
            } else {
                io::print_str(&alloc::format!("tar: cannot create {}\n", name));
            }
        }
        let blocks = (size + 511) / 512;
        offset += 512 + blocks as usize * 512;
    }
    0
    0
}

sarga_main!(user_main);
