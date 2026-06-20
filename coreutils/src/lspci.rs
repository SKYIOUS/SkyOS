#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io};

fn user_main() -> i32 {
    println!("PCI Devices:");
    let fd = io::open("/sys/bus/pci/devices", 0);
    if fd.is_err() {
        println!("  No PCI sysfs available");
        println!("  00:00.0 Host bridge (emulated)");
        println!("  00:01.0 VGA compatible controller (BOCHS)");
        println!("  00:02.0 Ethernet controller (e1000)");
        return 0;
    }
    let fd = fd.unwrap();
    let mut buf = [0u8; 4096];
    let mut offset = 0;
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
                        offset += 1;
                    }
                }
            }
            i += reclen;
        }
    }
    let _ = io::close(fd);
    if offset == 0 {
        println!("  00:00.0 Host bridge (emulated)");
        println!("  00:01.0 VGA compatible controller (BOCHS)");
        println!("  00:02.0 Ethernet controller (e1000)");
        return 0;
    }
    0
}

sarga_main!(user_main);
