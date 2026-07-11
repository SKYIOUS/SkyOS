#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, io};

fn file_exists(path: &str) -> bool {
    let fd = io::open(path, 0);
    if let Ok(fd) = fd {
        let _ = io::close(fd);
        true
    } else {
        false
    }
}

fn copy_file(src: &str, dst: &str) -> bool {
    let src_fd = match io::open(src, 0) {
        Ok(fd) => fd,
        Err(_) => return false,
    };
    let dst_fd = match io::open(dst, 0x41) { // O_WRONLY | O_CREAT
        Ok(fd) => fd,
        Err(_) => { let _ = io::close(src_fd); return false; }
    };
    let mut buf = [0u8; 8192];
    loop {
        match io::read(src_fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => { let _ = io::write_all(dst_fd, &buf[..n]); }
            Err(_) => break,
        }
    }
    let _ = io::close(src_fd);
    let _ = io::close(dst_fd);
    true
}

fn user_main() -> i32 {
    io::print_str("[skyd-update] daemon starting\n");

    // Check if an update is staged
    if file_exists("/tmp/update.toml") {
        io::print_str("[skyd-update] staged update found, applying...\n");

        if file_exists("/tmp/bin/sash") {
            io::print_str("[skyd-update] updating /bin/sash\n");
            copy_file("/tmp/bin/sash", "/bin/sash");
        }

        if file_exists("/tmp/bin/ade") {
            io::print_str("[skyd-update] updating /bin/ade\n");
            copy_file("/tmp/bin/ade", "/bin/ade");
        }

        // Cleanup
        let _ = io::unlink("/tmp/update.toml");
        io::print_str("[skyd-update] update applied successfully\n");
    } else {
        io::print_str("[skyd-update] no staged update found\n");
    }

    0
}

sarga_main!(user_main);
