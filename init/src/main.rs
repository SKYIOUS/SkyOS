#![no_std]
#![no_main]
extern crate alloc;
use core::sync::atomic::{AtomicBool, Ordering};
use libsarga::sarga_main;
use libsarga::io::*;
use libsarga::process::*;
use libsarga::syscall::*;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn sigchld_handler(_sig: u64) {
    loop {
        let r = unsafe { syscall2(61, 0, 0x00000001) };
        if (r as i64) < 0 { break; }
    }
}

extern "C" fn sighup_handler(_sig: u64) {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

fn sig_ignore(sig: u64) {
    let ign: u64 = 1;
    unsafe { syscall3(13, sig, &ign as *const u64 as u64, 0); }
}

fn sig_handle(sig: u64, handler: u64) {
    unsafe { syscall3(13, sig, 0, handler); }
}

fn read_config() -> (alloc::vec::Vec<u8>, bool) {
    let mut config = alloc::vec::Vec::new();
    let mut has_config = false;
    if let Ok(fd) = open("/etc/init.cfg", 0) {
        let mut buf = [0u8; 4096];
        if let Ok(n) = read(fd, &mut buf) {
            if n > 0 {
                config.extend_from_slice(&buf[..n]);
                has_config = true;
            }
        }
        let _ = close(fd);
    }
    (config, has_config)
}

fn parse_config(config: &[u8]) -> (alloc::vec::Vec<(u8, alloc::vec::Vec<u8>)>, alloc::vec::Vec<u8>) {
    use alloc::vec::Vec;
    let mut services: Vec<(u8, Vec<u8>)> = Vec::new();
    let mut login = Vec::from(b"/bin/sash");
    if config.is_empty() { return (services, login); }
    if let Ok(s) = core::str::from_utf8(config) {
        for line in s.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            if let Some(rest) = line.strip_prefix("login ") {
                let p = rest.trim();
                if !p.is_empty() { login = Vec::from(p.as_bytes()); }
            } else if let Some(rest) = line.strip_prefix("service ") {
                if let Some((level_str, path)) = rest.split_once(' ') {
                    if let Ok(level) = level_str.parse::<u8>() {
                        let p = path.trim();
                        if !p.is_empty() { services.push((level, Vec::from(p.as_bytes()))); }
                    }
                }
            }
        }
    }
    (services, login)
}

fn start_services(services: &[(u8, alloc::vec::Vec<u8>)], runlevel: u8) {
    for (level, path) in services {
        if *level == runlevel {
            match fork() {
                Ok(0) => {
                    let arg = core::str::from_utf8(path).unwrap_or("");
                    let args = [arg];
                    let _ = execve(arg, &args, &[]);
                    exit(1);
                }
                Ok(pid) => { let _ = wait(pid); }
                Err(_) => {}
            }
        }
    }
}

fn user_main() {
    write_all(1, b"[init] started\n").ok();
    sig_ignore(17);
    sig_handle(1, sigchld_handler as *const () as u64);
    let (config, _) = read_config();
    let (services, login_path_bytes) = parse_config(&config);
    let login_path = core::str::from_utf8(&login_path_bytes).unwrap_or("/bin/sash").trim_end_matches('\0');
    start_services(&services, 1);
    loop {
        if SHUTDOWN.load(Ordering::SeqCst) { break; }
        match fork() {
            Ok(0) => {
                sig_handle(1, sighup_handler as *const () as u64);
                let args = [login_path];
                let _ = execve(login_path, &args, &[]);
                write_all(1, b"init: execve failed\n").ok();
                exit(1);
            }
            Ok(pid) => { let _ = wait(pid); }
            Err(_) => {
                write_all(1, b"init: fork failed\n").ok();
                break;
            }
        }
    }
    write_all(1, b"init: shutting down\n").ok();
    unsafe { syscall1(60, 0); }
    loop { core::hint::spin_loop(); }
}

sarga_main!(user_main);
