#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println};

fn user_main() {
    println!("Filesystem     Size  Used  Avail  Mount");
    println!("/dev/initrd    16M   5.2M  10.8M  /");
    println!("tmpfs          64M   0     64M    /tmp");
}

sarga_main!(user_main);
