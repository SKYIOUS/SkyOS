#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, ai, io, syscall::*};
use alloc::string::String;

fn update_proc_file(path: &str, query: &str) {
    if let Ok(info) = ai::query(query) {
        if let Ok(fd) = io::open(path, 0x2 | 0x40) { // O_RDWR | O_CREAT
            let _ = io::write(fd, info.as_bytes());
            let _ = io::close(fd);
        }
    }
}

fn user_main() {
    println!("Sarga /proc daemon (procd) started");
    
    // Create /proc structure
    let _ = io::open("/proc/cpuinfo", 0x40);
    let _ = io::open("/proc/meminfo", 0x40);
    let _ = io::open("/proc/uptime", 0x40);
    let _ = io::open("/proc/version", 0x40);

    loop {
        // Populate system info using the VahiAI bridge
        update_proc_file("/proc/cpuinfo", "report cpu model, cores, and MHz raw format");
        update_proc_file("/proc/meminfo", "report memory usage raw MemTotal MemFree MemAvailable Cached");
        update_proc_file("/proc/uptime", "report system uptime since boot in seconds");
        update_proc_file("/proc/version", "report kernel version string");
        
        // Sleep for 5 seconds between updates (syscall 35 nanosleep)
        unsafe {
            let req = [5u64, 0u64]; // 5 seconds, 0 nanoseconds
            syscall2(35, req.as_ptr() as u64, 0);
        }
    }
}

sarga_main!(user_main);
