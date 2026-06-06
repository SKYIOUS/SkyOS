#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io, args};

const DB_DIR: &str = "/var/spkg/db";

fn ensure_db() {
    unsafe { libsarga::syscall::syscall3(83, DB_DIR.as_ptr() as u64, 0o755, 0); }
}

fn cmd_install() {
    if args::argc() < 2 { println!("Usage: spkg install <package-dir>"); return; }
    let pkg_dir = args::get(1).unwrap_or("");
    let name = pkg_dir.trim_end_matches('/').split('/').last().unwrap_or(pkg_dir);
    let db_path = alloc::format!("{}/{}", DB_DIR, name);
    unsafe { libsarga::syscall::syscall3(83, db_path.as_ptr() as u64, 0o755, 0); }
    let info_path = alloc::format!("{}/info", db_path);
    let fd = unsafe { libsarga::syscall::syscall2(2, info_path.as_ptr() as u64, 0x42 | 0o644) };
    if (fd as i64) < 0 { println!("spkg: failed to create info"); return; }
    let manifest = alloc::format!("name: {}\nfiles: {}\n", name, pkg_dir);
    unsafe { libsarga::syscall::syscall3(1, fd as u64, manifest.as_ptr() as u64, manifest.len() as u64); }
    unsafe { libsarga::syscall::syscall1(3, fd as u64); }
    println!("spkg: installed {}", name);
}

fn cmd_remove() {
    if args::argc() < 2 { println!("Usage: spkg remove <package>"); return; }
    let name = args::get(1).unwrap_or("");
    let db_path = alloc::format!("{}/{}", DB_DIR, name);
    let r = unsafe { libsarga::syscall::syscall1(87, db_path.as_ptr() as u64) };
    if r != 0 { println!("spkg: {} not found", name); }
    else { println!("spkg: removed {}", name); }
}

fn cmd_list() {
    let fd = match io::open(DB_DIR, 0) {
        Ok(fd) => fd,
        Err(_) => { println!("spkg: no packages installed"); return; }
    };
    let mut buf = [0u8; 4096];
    let mut found = false;
    loop {
        let r = unsafe { libsarga::syscall::syscall3(78, fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (r as i64) <= 0 { break; }
        let entries = &buf[..r as usize];
        let mut i = 0;
        while i + 16 <= entries.len() {
            let reclen = u16::from_ne_bytes([entries[i+8], entries[i+9]]) as usize;
            let namelen = entries[i+10] as usize;
            if reclen < 11 || i + reclen > entries.len() { break; }
            if namelen > 0 && i + 11 + namelen <= entries.len() {
                if let Ok(name) = core::str::from_utf8(&entries[i+11..i+11+namelen]) {
                    if name != "." && name != ".." {
                        println!("  {}", name);
                        found = true;
                    }
                }
            }
            i += reclen;
        }
    }
    let _ = io::close(fd);
    if !found { println!("spkg: no packages installed"); }
}

fn user_main() {
    if args::argc() < 2 { println!("Usage: spkg <install|remove|list|update> [args]"); return; }
    ensure_db();
    match args::get(1).unwrap_or("") {
        "install" => cmd_install(),
        "remove" => cmd_remove(),
        "list" | "ls" => cmd_list(),
        "update" => println!("spkg: update not yet implemented"),
        _ => println!("spkg: unknown command"),
    }
}

sarga_main!(user_main);
