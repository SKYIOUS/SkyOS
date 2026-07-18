#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;
use libsarga::sarga_main;
use libsarga::io::{self, open, close, getdents64, stat, Stat};
use libsarga::fs;

fn walk_dir(path: &str, entries: &mut Vec<(String, String, u64, bool)>, depth: usize) {
    if depth > 8 { return; }
    if path.len() > 256 { return; }
    let fd = match open(path, 0) { Ok(f) => f, Err(_) => return };
    let mut buf = [0u8; 4096];
    let n = match getdents64(fd, &mut buf) { Ok(n) => n, Err(_) => { let _ = close(fd); return } };
    let _ = close(fd);

    let mut off = 0;
    while off < n {
        if off + 19 > n { break; }
        let reclen = u16::from_le_bytes(buf[off+16..off+18].try_into().unwrap_or([0; 2])) as usize;
        let d_type = buf[off+18];
        if reclen < 19 || off + reclen > n { break; }
        let name_end = buf[off+19..off+reclen].iter().position(|&b| b == 0).unwrap_or(reclen - 19);
        let name = core::str::from_utf8(&buf[off+19..off+19+name_end]).unwrap_or("");
        if name.is_empty() || name == "." || name == ".." { off += reclen; continue; }

        let full = if path == "/" { format!("/{}", name) } else { format!("{}/{}", path, name) };
        let is_dir = d_type == 4;
        let size = stat(&full).map(|s: Stat| s.size as u64).unwrap_or(0);
        let full_clone = full.clone();
        entries.push((name.to_string(), full_clone, size, is_dir));

        if is_dir && depth < 8 {
            walk_dir(&full, entries, depth + 1);
        }
        off += reclen;
    }
}

fn build_index() -> Vec<(String, String, u64, bool)> {
    let mut entries = Vec::new();
    walk_dir("/", &mut entries, 0);
    entries
}

fn write_index(entries: &[(String, String, u64, bool)]) {
    let mut out = String::new();
    for (name, path, size, is_dir) in entries {
        out.push_str(&format!("{}|{}|{}|{}\n", name, path, size, *is_dir as u8));
    }
    let _ = fs::write_file("/tmp/search.idx", &out);
}

fn user_main() -> i32 {
    io::print_str("[searchd] starting filesystem indexer\n");
    let mut cycle = 0u64;

    loop {
        io::print_str(&format!("[searchd] indexing... (cycle {})\n", cycle));
        let entries = build_index();
        write_index(&entries);
        io::print_str(&format!("[searchd] indexed {} entries\n", entries.len()));

        cycle += 1;
        for _ in 0..30000 {
            unsafe { libsarga::syscall::syscall2(35, 0, 1_000_000u64); }
        }
    }
}

sarga_main!(user_main);
