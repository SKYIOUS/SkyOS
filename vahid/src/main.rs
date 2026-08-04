#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use libsarga::io::{self, close, open};
use libsarga::sarga_main;

const PCI_DEVS: &str = "/sys/bus/pci/devices/";

// allow: PCI vendor/device name table reserved for future device reporting; type is fixed and read-only
#[allow(dead_code, clippy::type_complexity)]
const VENDORS: &[(u16, &str, &[(u16, &str)])] = &[
    (
        0x8086,
        "Intel Corporation",
        &[
            (0x100e, "82540EM Gigabit Ethernet"),
            (0x1237, "440FX - 82441FX PMC"),
            (0x7000, "82371SB PIIX3 ISA"),
            (0x7010, "82371SB PIIX3 IDE"),
            (0x7111, "82371AB/EB/MB PIIX4 IDE"),
            (0x7113, "82371AB/EB/MB PIIX4 ACPI"),
            (0x29c0, "82G33/G31/P35/P31 Express DRAM"),
        ],
    ),
    (
        0x10ec,
        "Realtek Semiconductor",
        &[(0x8139, "RTL-8139/8139C/8139C+")],
    ),
    (
        0x1af4,
        "Red Hat / QEMU",
        &[
            (0x1000, "Virtio Network Device"),
            (0x1001, "Virtio Block Device"),
        ],
    ),
];

// allow: reserved for future device reporting
#[allow(dead_code)]
fn vendor_name(vid: u16) -> &'static str {
    for (v, name, _) in VENDORS {
        if *v == vid {
            return name;
        }
    }
    "Unknown Vendor"
}

// allow: reserved for future device reporting
#[allow(dead_code)]
fn device_name(vid: u16, did: u16) -> &'static str {
    for (v, _, devices) in VENDORS {
        if *v == vid {
            for (d, name) in *devices {
                if *d == did {
                    return name;
                }
            }
        }
    }
    "Unknown Device"
}

fn scan_pci() {
    match open(PCI_DEVS, 0) {
        Ok(fd) => {
            io::print_str("[vahid] Scanning PCI...\n");
            let mut buf = [0u8; 4096];
            let n = io::getdents64(fd, &mut buf).unwrap_or(0);
            let mut off = 0;
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
                }
                if reclen == 0 {
                    break;
                }
                off += reclen;
            }
            let _ = close(fd);
        }
        Err(_) => {
            io::print_str("[vahid] sysfs not found, skipping scan\n");
        }
    }
}

/// Try mknod-style fallback: write device path hints for kernel to pick up.
fn create_devices() {
    let nodes: &[(&str, u32, u32)] = &[
        ("null", 1, 3),
        ("zero", 1, 5),
        ("random", 1, 8),
        ("urandom", 1, 9),
        ("tty", 5, 0),
        ("console", 5, 1),
    ];
    for (name, major, minor) in nodes {
        let path = alloc::format!("/dev/{}", name);
        // Try mknod via raw syscall; fall back to O_CREAT if unavailable
        let ret = unsafe {
            libsarga::syscall::syscall3(
                0x7d,
                path.as_ptr() as u64,
                0x2000 | *major as u64,
                *minor as u64,
            )
        };
        if ret < 0 {
            let _ = open(&path, 0x41); // O_CREAT|O_WRONLY
        }
    }
}

fn user_main() -> i32 {
    io::print_str("[vahid] SkyOS Device Manager\n");
    scan_pci();
    create_devices();
    io::print_str("[vahid] ready\n");
    loop {
        let _ = io::nanosleep(1_000_000_000);
    }
}

sarga_main!(user_main);
