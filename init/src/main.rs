#![no_std]
#![no_main]

extern crate alloc;
use libsarga::sarga_main;
use libsarga::io::*;
use libsarga::process::*;

sarga_main!(user_main);

fn sig_ignore(sig: u64) {
    let ign: u64 = 1;
    unsafe { libsarga::syscall::syscall3(13, sig, &ign as *const u64 as u64, 0); }
}

fn get_login_path() -> [u8; 64] {
    let default = b"/bin/sash";
    let mut buf = [0u8; 64];
    let fd = open("/etc/init.cfg", 0);
    if fd.is_err() {
        buf[..default.len()].copy_from_slice(default);
        return buf;
    }
    let fd = fd.unwrap();
    let mut file_buf = [0u8; 1024];
    let n = read(fd, &mut file_buf).unwrap_or(0);
    let _ = close(fd);
    if n > 0 {
        if let Ok(s) = core::str::from_utf8(&file_buf[..n]) {
            for line in s.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') { continue; }
                if let Some(rest) = line.strip_prefix("login ") {
                    let p = rest.trim();
                    if !p.is_empty() {
                        let len = p.len().min(63);
                        buf[..len].copy_from_slice(p.as_bytes());
                        return buf;
                    }
                }
            }
        }
    }
    buf[..default.len()].copy_from_slice(default);
    buf
}

fn user_main() {
    write_all(1, b"[init] started\n").ok();
    sig_ignore(17);
    write_all(1, b"[init] sig_ignore done\n").ok();
    let login_buf = get_login_path();
    write_all(1, b"[init] login_path=").ok();
    write_all(1, &login_buf[..login_buf.iter().position(|&c| c==0).unwrap_or(64)]).ok();
    write_all(1, b"\n").ok();
    loop {
        write_all(1, b"[init] forking...\n").ok();
        let pid = match fork() {
            Ok(0) => {
                let path = core::str::from_utf8(&login_buf).unwrap_or("/bin/sash").trim_end_matches('\0');
                let args = [path];
                let _ = execve(path, &args, &[]);
                write_all(1, b"init: execve failed\n").ok();
                exit(1);
            }
            Ok(pid) => pid,
            Err(_) => {
                write_all(1, b"init: fork failed\n").ok();
                break;
            }
        };
        let _ = wait(pid);
        write_all(1, b"init: shell exited, respawning...\n").ok();
    }
    loop { core::hint::spin_loop(); }
}
