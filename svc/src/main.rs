#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use libsarga::args;
use libsarga::io::{self, close, getdents64, open};
use libsarga::process::{execve, exit, fork, kill, waitpid};
use libsarga::sarga_main;

fn eprint(s: &str) {
    io::print_str(s);
}

fn format_pid_list() -> Vec<(u64, String)> {
    let dir_fd = match open("/proc", 0) {
        Ok(fd) => fd,
        Err(_) => return vec![],
    };

    let mut buf = [0u8; 4096];
    let n = match getdents64(dir_fd, &mut buf) {
        Ok(n) => n,
        Err(_) => {
            let _ = close(dir_fd);
            return vec![];
        }
    };
    let _ = close(dir_fd);

    let mut result = vec![];
    let mut off = 0;
    while off < n {
        let d_ino = u64::from_ne_bytes(buf[off..off + 8].try_into().unwrap_or([0; 8]));
        let d_reclen =
            u16::from_ne_bytes(buf[off + 16..off + 18].try_into().unwrap_or([0; 2])) as usize;
        let name_start = off + 19;
        let name_end = buf[name_start..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| name_start + p)
            .unwrap_or(off + d_reclen);

        if d_ino != 0 && name_start < buf.len() {
            let name =
                core::str::from_utf8(&buf[name_start..name_end.min(buf.len())]).unwrap_or("");
            if let Ok(pid) = name.parse::<u64>() {
                let cmd_path = alloc::format!("/proc/{}/cmdline", pid);
                if let Ok(cmd) = io::read_to_string(&cmd_path) {
                    result.push((pid, cmd.trim_end_matches('\0').to_string()));
                }
            }
        }
        if d_reclen == 0 {
            break;
        }
        off += d_reclen;
    }
    result
}

fn cmd_status() {
    let procs = format_pid_list();
    eprint("PID   COMMAND\n");
    for (pid, cmd) in &procs {
        let line = alloc::format!("{:<5} {}\n", pid, cmd);
        eprint(&line);
    }
}

fn cmd_start(path: &str) {
    match fork() {
        Ok(0) => {
            let _ = execve(path, &[], &[]);
            exit(1);
        }
        Ok(pid) => {
            eprint(&alloc::format!("[svc] started PID {}\n", pid));
        }
        Err(_) => eprint("[svc] fork failed\n"),
    }
}

fn cmd_stop(path: &str) {
    let procs = format_pid_list();
    for (pid, cmd) in &procs {
        if cmd == path || cmd.ends_with(path) {
            eprint(&alloc::format!("[svc] stopping PID {}...\n", pid));
            let _ = kill(*pid as i64, 15);
            let mut stopped = false;
            for _ in 0..50 {
                if let Ok((reaped, _)) = waitpid(*pid as i64, 1) {
                    if reaped != 0 {
                        stopped = true;
                        break;
                    }
                }
                let _ = io::nanosleep(100_000_000);
            }
            if stopped {
                eprint("[svc] stopped\n");
            } else {
                let _ = kill(*pid as i64, 9);
                let _ = waitpid(*pid as i64, 0);
                eprint("[svc] stopped (SIGKILL after timeout)\n");
            }
            return;
        }
    }
    eprint("[svc] not found\n");
}

fn user_main() -> i32 {
    let argc = args::argc();
    if argc < 2 {
        eprint("Usage: svc <status|start|stop|restart> [path]\n");
        return 1;
    }

    let cmd = args::get(1).unwrap_or("");
    match cmd {
        "status" | "list" | "ls" => cmd_status(),
        "start" => {
            if argc < 3 {
                eprint("missing path\n");
                return 1;
            }
            cmd_start(args::get(2).unwrap_or(""));
        }
        "stop" => {
            if argc < 3 {
                eprint("missing path\n");
                return 1;
            }
            cmd_stop(args::get(2).unwrap_or(""));
        }
        "restart" => {
            if argc < 3 {
                eprint("missing path\n");
                return 1;
            }
            let p = args::get(2).unwrap_or("");
            cmd_stop(p);
            cmd_start(p);
        }
        _ => {
            eprint("unknown command\n");
            return 1;
        }
    }
    0
}

sarga_main!(user_main);
