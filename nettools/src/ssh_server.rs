#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use libsarga::io::{read, write};
use libsarga::net::{self, accept, bind, listen, socket, SockAddrIn};
use libsarga::process::{execve, exit, fork};
use libsarga::sarga_main;

fn handle_client(fd: i64) {
    let mut buf = [0u8; 4096];
    let _ = write(fd, b"SkyOS SSH Server (raw shell)\r\nlogin: ");
    let n = read(fd, &mut buf).unwrap_or(0);
    if n == 0 {
        return;
    }
    match fork() {
        Ok(0) => {
            // ponytail: shell on port 2222, execve /bin/sh
            let _ = execve("/bin/sh", &[], &[]);
            exit(1);
        }
        Ok(_) => {
            let _ = libsarga::io::close(fd);
        }
        Err(_) => {}
    }
}

fn user_main() -> i32 {
    let port = 2222;
    let fd = match socket(net::AF_INET, net::SOCK_STREAM, 0) {
        Ok(f) => f,
        Err(_) => return 1,
    };
    let addr = SockAddrIn::new([0, 0, 0, 0], port);
    if bind(fd, &addr).is_err() {
        return 1;
    }
    if listen(fd, 5).is_err() {
        return 1;
    }
    loop {
        if let Ok((client_fd, _)) = accept(fd) {
            handle_client(client_fd);
        }
    }
}

sarga_main!(user_main);
