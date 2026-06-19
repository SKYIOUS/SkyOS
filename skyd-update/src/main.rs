#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, io, fs, process};

fn user_main() -> i32 {
    io::print_str("[skyd-update] daemon starting\n");

    // Check if an update is staged
    if fs::exists("/tmp/update.toml") {
        io::print_str("[skyd-update] staged update found, applying...\n");

        // This is where the actual replacement logic would go
        // For now, we mock the process of moving files from /tmp to /bin
        if fs::exists("/tmp/bin/sash") {
            io::print_str("[skyd-update] updating /bin/sash\n");
            let _ = fs::copy("/tmp/bin/sash", "/bin/sash");
        }

        if fs::exists("/tmp/bin/ade") {
            io::print_str("[skyd-update] updating /bin/ade\n");
            let _ = fs::copy("/tmp/bin/ade", "/bin/ade");
        }

        // Cleanup
        let _ = fs::remove("/tmp/update.toml");
        io::print_str("[skyd-update] update applied successfully\n");
    } else {
        io::print_str("[skyd-update] no staged update found\n");
    }

    0
}

sarga_main!(user_main);
