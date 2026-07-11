#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use alloc::ffi::CString;
use libsarga::sarga_main;
use libsarga::io::{self, open, close};
use libsarga::syscall::*;

const PCI_DEVS: &str = "/sys/bus/pci/devices/";

const VENDORS: &[(u16, &'static str, &[(u16, &'static str)])] = &[
    (0x8086, "Intel Corporation", &[
        (0x100e, "82540EM Gigabit Ethernet"),
        (0x1237, "440FX - 82441FX PMC"),
        (0x7000, "82371SB PIIX3 ISA"),
        (0x7010, "82371SB PIIX3 IDE"),
        (0x7111, "82371AB/EB/MB PIIX4 IDE"),
        (0x7113, "82371AB/EB/MB PIIX4 ACPI"),
        (0x29c0, "82G33/G31/P35/P31 Express DRAM"),
    ]),
    (0x10ec, "Realtek Semiconductor", &[(0x8139, "RTL-8139/8139C/8139C+")]),
    (0x1af4, "Red Hat / QEMU", &[(0x1000, "Virtio Network Device"), (0x1001, "Virtio Block Device")]),
];

fn vendor_name(vid: u16) -> &'static str {
    for (v, name, _) in VENDORS { if *v == vid { return name; } }
    "Unknown Vendor"
}

fn device_name(vid: u16, did: u16) -> &'static str {
    for (v, _, devices) in VENDORS {
        if *v == vid {
            for (d, name) in *devices { if *d == did { return name; } }
        }
    }
    "Unknown Device"
}

fn scan_pci() {
    match open(PCI_DEVS, 0) {
        Ok(fd) => {
            io::print_str("[vahid] Scanning PCI...\n");
            let _ = close(fd);
        }
        Err(_) => {
            io::print_str("[vahid] sysfs not found, skipping scan\n");
        }
    }
}

fn create_devices() {
    let nodes = &["null", "zero", "random", "urandom", "tty", "console"];
    for name in nodes {
        let path = alloc::format!("/dev/{}", name);
        // O_CREAT = 0x40, O_WRONLY = 0x01 -> 0x41
        if let Ok(fd) = open(&path, 0x41) {
            let _ = close(fd);
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
