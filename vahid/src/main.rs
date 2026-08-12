#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use libsarga::io::{self, close, open};
use libsarga::sarga_main;

const PCI_DEVS: &str = "/sys/bus/pci/devices/";

/// Exit code for a fatal device-scan failure. Deliberately NON-ZERO so
/// init's crash accounting (init/src/main.rs) counts this as a crash and
/// eventually gives up after MAX_RESPAWNS, instead of treating it as a clean
/// exit (status 0), which resets the crash counter and respawns forever.
/// A healthy vahid never exits (infinite sleep loop), so this code is only
/// reachable when device setup genuinely failed.
const EXIT_DEVICE_SCAN_FAILED: i32 = 1;

/// Enumerate PCI devices under /sys/bus/pci/devices/.
///
/// Returns the number of devices found, or None when sysfs is unavailable
/// (degraded but not fatal: device nodes are still created below, and the
/// kernel is in major change so sysfs may not be populated yet).
fn scan_pci() -> Option<usize> {
    match open(PCI_DEVS, 0) {
        Ok(fd) => {
            io::print_str("[vahid] Scanning PCI...\n");
            let mut buf = [0u8; 4096];
            let n = io::getdents64(fd, &mut buf).unwrap_or(0);
            let mut off = 0;
            let mut count = 0usize;
            while off + 19 < n {
                let reclen =
                    u16::from_le_bytes(buf[off + 16..off + 18].try_into().unwrap_or([0; 2]))
                        as usize;
                if reclen < 19 || off + reclen > n {
                    break;
                }
                let name_end = buf[off + 19..off + reclen]
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(reclen - 19);
                let name = core::str::from_utf8(&buf[off + 19..off + 19 + name_end]).unwrap_or("");
                if !name.is_empty() && name != "." && name != ".." {
                    io::print_str(&alloc::format!("  PCI device: {}\n", name));
                    count += 1;
                }
                if reclen == 0 {
                    break;
                }
                off += reclen;
            }
            let _ = close(fd);
            Some(count)
        }
        Err(_) => {
            io::print_str("[vahid] sysfs not found, skipping scan\n");
            None
        }
    }
}

/// Create the standard device nodes. Returns true when every node exists.
///
/// This is the system-critical output: without /dev/null, /dev/zero, etc.
/// the OS cannot function. A failure here is the FATAL condition.
fn create_devices() -> bool {
    let nodes: &[(&str, u32, u32)] = &[
        ("null", 1, 3),
        ("zero", 1, 5),
        ("random", 1, 8),
        ("urandom", 1, 9),
        ("tty", 5, 0),
        ("console", 5, 1),
    ];
    let mut all_ok = true;
    for (name, _major, _minor) in nodes {
        let path = alloc::format!("/dev/{}", name);
        // Node creation is the O_CREAT fallback alone (open(path, O_CREAT |
        // O_WRONLY)) — see session-lifecycle.md for the 0x7d removal
        // decision: syscall 0x7d is SYS_CLIPBOARD (125), not mknod, so it
        // could never create a node and its result was discarded anyway.
        // When the kernel grows a real mknod, add it back GATED on its
        // result and verify the node instead of calling it unconditionally.
        if open(&path, 0x41).is_err() {
            io::print_str(&alloc::format!("[vahid] FAILED to create /dev/{}\n", name));
            all_ok = false;
        }
    }
    all_ok
}

fn user_main() -> i32 {
    io::print_str("[vahid] SkyOS Device Manager\n");
    let devices = scan_pci();
    if !create_devices() {
        // FATAL: the device nodes the system depends on could not be
        // created. Exit non-zero so init can distinguish this from a
        // healthy sleep loop and give up after MAX_RESPAWNS.
        io::print_str("[vahid] FATAL: failed to create device nodes\n");
        libsarga::process::exit(EXIT_DEVICE_SCAN_FAILED);
    }
    match devices {
        Some(n) => io::print_str(&alloc::format!("[vahid] scanned {} PCI device(s)\n", n)),
        None => io::print_str("[vahid] sysfs unavailable, continuing without scan\n"),
    }
    io::print_str("[vahid] ready\n");
    // Healthy sleep loop: never exits, so init never sees an exit event and
    // never respawns a working vahid.
    loop {
        let _ = io::nanosleep(1_000_000_000);
    }
}

sarga_main!(user_main);
