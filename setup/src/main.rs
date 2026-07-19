#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use libsarga::{args, io, process, sarga_main};

fn user_main() -> i32 {
    io::print_str("========================================\n");
    io::print_str("  SARGA OS Installer v1.1\n");
    io::print_str("========================================\n\n");

    io::print_str("This will prepare SARGA OS for first use.\n");

    if process::getuid() != 0 {
        io::print_str("Error: Setup must be run as root.\n");
        return 1;
    }

    io::print_str("Creating system directories...\n");
    let dirs = [
        "/etc", "/bin", "/tmp", "/var", "/home", "/root", "/usr", "/usr/bin", "/usr/lib",
    ];
    for d in &dirs {
        let _ = io::mkdir(d, 0o755);
    }

    io::print_str("Setting up basic configuration...\n");
    if let Err(e) = libsarga::fs::write_file("/etc/hostname", "sarga-os\n") {
        io::print_str(&alloc::format!(
            "Warning: Failed to write /etc/hostname: {}\n",
            e
        ));
    }

    if let Err(e) = libsarga::fs::write_file("/etc/passwd", "root:x:0:0:root:/root:/bin/sash\n") {
        io::print_str(&alloc::format!(
            "Warning: Failed to write /etc/passwd: {}\n",
            e
        ));
    }

    io::print_str("\nSetup complete. Welcome to SARGA OS!\n");
    0
}

sarga_main!(user_main);
