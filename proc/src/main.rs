#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io, syscall::*};

fn copy_ctl_to_proc(ctl_path: &str, proc_path: &str) {
    if let Ok(data) = io::read_to_string(ctl_path) {
        if let Ok(fd) = io::open(proc_path, 0x2 | 0x40) {
            let _ = io::write_all(fd, data.as_bytes());
            let _ = io::close(fd);
        }
    }
}

fn user_main() -> i32 {
    println!("Sarga /proc daemon (procd) started");

    loop {
        // Copy real kernel data from /ctl to /proc
        copy_ctl_to_proc("/ctl/sys/cpu/info", "/proc/cpuinfo");
        copy_ctl_to_proc("/ctl/sys/mem/total", "/proc/meminfo");
        copy_ctl_to_proc("/ctl/kernel/uptime", "/proc/uptime");
        copy_ctl_to_proc("/ctl/kernel/version", "/proc/version");
        copy_ctl_to_proc("/ctl/proc/list", "/proc/self/status");
        copy_ctl_to_proc("/ctl/sys/net/eth0/addr", "/proc/net/if_inet6");
        copy_ctl_to_proc("/ctl/sys/net/stat", "/proc/net/sockstat");

        // Sleep 2 seconds between updates
        unsafe {
            let req = [2u64, 0u64];
            syscall2(35, req.as_ptr() as u64, 0);
        }
    }
}

sarga_main!(user_main);
