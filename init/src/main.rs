#![no_std]
#![no_main]
extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use libsarga::sarga_main;
use libsarga::io::*;
use libsarga::process::*;
use libsarga::syscall::*;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static REBOOT: AtomicBool = AtomicBool::new(false);

struct Service {
    name: String,
    exec: String,
    respawn: bool,
    pid: u64,
    running: bool,
}

// ── helpers ─────────────────────────────────────────────────────────────────

fn mount_fs(fstype: &str, target: &str) {
    let src_c = alloc::format!("{}\0", "none");
    let tgt_c = alloc::format!("{}\0", target);
    let fs_c = alloc::format!("{}\0", fstype);
    let r = unsafe { syscall4(165, src_c.as_ptr() as u64, tgt_c.as_ptr() as u64, fs_c.as_ptr() as u64, 0) };
    if r < 0 {
        write_all(1, alloc::format!("[init] mount {} on {} failed: {}\n", fstype, target, -r).as_bytes()).ok();
    } else {
        write_all(1, alloc::format!("[init] mounted {} on {}\n", fstype, target).as_bytes()).ok();
    }
}

fn read_whole_file(path: &str) -> Result<Vec<u8>, i64> {
    let path_c = alloc::format!("{}\0", path);
    let fd = open(&path_c, 0)?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = read(fd, &mut tmp)?;
        if n == 0 { break; }
        buf.extend_from_slice(&tmp[..n]);
    }
    close(fd)?;
    Ok(buf)
}

// ── config parser (simple /etc/init.toml) ───────────────────────────────────

fn parse_config(data: &[u8]) -> (String, Vec<(String, String, bool)>) {
    let mut hostname = String::from("sargathos");
    let mut services: Vec<(String, String, bool)> = Vec::new();
    let mut in_service = false;
    let mut current_name = String::new();
    let mut current_exec = String::new();
    let mut current_respawn = false;

    let text = core::str::from_utf8(data).unwrap_or("");
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        if line.starts_with('[') && line.ends_with(']') {
            // flush previous service
            if in_service {
                services.push((current_name.clone(), current_exec.clone(), current_respawn));
                current_name.clear();
                current_exec.clear();
                current_respawn = false;
            }
            in_service = line == "[[service]]";
            continue;
        }

        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let value = line[eq+1..].trim().trim_matches('"');
            if !in_service {
                if key == "hostname" {
                    hostname = value.to_string();
                }
            } else {
                match key {
                    "name" => current_name = value.to_string(),
                    "exec" => current_exec = value.to_string(),
                    "respawn" => current_respawn = value == "true",
                    _ => {}
                }
            }
        }
    }
    if in_service {
        services.push((current_name, current_exec, current_respawn));
    }
    (hostname, services)
}

// ── signal handling ─────────────────────────────────────────────────────────

extern "C" fn sigchld_handler(_sig: u64) {
    // Reap all exited children
    loop {
        let r = unsafe { syscall2(61, 0, 0x00000001) }; // wait4(-1, WNOHANG)
        if (r as i64) < 0 { break; }
        if r == 0 { break; } // no more children
    }
}

extern "C" fn sigterm_handler(_sig: u64) {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

fn set_signal(sig: u64, handler: u64) {
    // SYS_RT_SIGACTION: syscall3(13, sig, act, oldact)
    // act is a pointer to the handler value in userspace
    let handler_val = handler;
    unsafe { syscall3(13, sig, &handler_val as *const u64 as u64, 0); }
}

// ── service management ──────────────────────────────────────────────────────

fn start_service(svc: &Service) -> Option<u64> {
    match fork() {
        Ok(0) => {
            // Child: set default signal handlers, exec
            let exec_path = svc.exec.clone();
            let args = [exec_path.as_str()];
            let env = ["TERM=xterm-256color"];
            execve(exec_path.as_str(), &args, &env);
            write_all(1, alloc::format!("[init] failed to exec {}\n", svc.exec).as_bytes()).ok();
            exit(1);
        }
        Ok(pid) => {
            write_all(1, alloc::format!("[init] started {} (pid {})\n", svc.name, pid).as_bytes()).ok();
            Some(pid)
        }
        Err(_) => {
            write_all(1, alloc::format!("[init] fork failed for {}\n", svc.name).as_bytes()).ok();
            None
        }
    }
}

fn shutdown_all(services: &mut BTreeMap<String, Service>, do_reboot: bool) {
    write_all(1, b"[init] shutting down...\n").ok();

    // SIGTERM all services
    for (_, svc) in services.iter() {
        if svc.running {
            kill(svc.pid as i64, 15); // SIGTERM
        }
    }

    // Wait up to 5 seconds
    for _ in 0..50 {
        let mut all_dead = true;
        // reap any exited children
        loop {
            let r = unsafe { syscall2(61, 0, 0x00000001) };
            if (r as i64) < 0 { break; }
            if r == 0 { break; }
            // Mark service as not running
            for (_, svc) in services.iter_mut() {
                if svc.running && svc.pid == r as u64 {
                    svc.running = false;
                    write_all(1, alloc::format!("[init] {} exited\n", svc.name).as_bytes()).ok();
                }
            }
        }
        for (_, svc) in services.iter() {
            if svc.running { all_dead = false; break; }
        }
        if all_dead { break; }
        unsafe { syscall1(35, 100_000_000); } // nanosleep 100ms
    }

    // SIGKILL remaining
    for (name, svc) in services.iter() {
        if svc.running {
            write_all(1, alloc::format!("[init] force killing {}...\n", name).as_bytes()).ok();
            kill(svc.pid as i64, 9);
        }
    }

    // Sync and reboot/poweroff
    let _ = sync();
    if do_reboot {
        write_all(1, b"[init] rebooting...\n").ok();
        reboot();
    } else {
        write_all(1, b"[init] powering off...\n").ok();
        reboot(); // 0 = poweroff
    }
    loop { core::hint::spin_loop(); }
}

// ── main ────────────────────────────────────────────────────────────────────

fn user_main() {
    write_all(1, b"[init] Vahi init v0.1 starting\n").ok();

    // Mount filesystems
    mount_fs("devfs", "/dev");
    mount_fs("tmpfs", "/tmp");
    mount_fs("ctlfs", "/ctl");

    // Set hostname via /ctl
    let hostname = {
        let data = match read_whole_file("/etc/init.toml") {
            Ok(d) => d,
            Err(_) => Vec::new(),
        };
        let (h, _) = parse_config(&data);
        h
    };
    write_all(1, alloc::format!("[init] hostname: {}\n", hostname).as_bytes()).ok();

    // Set up signal handlers
    set_signal(2, 1);            // SIGINT → ignore (1 = SIG_IGN)
    set_signal(15, sigterm_handler as *const () as u64); // SIGTERM → shutdown
    set_signal(17, sigchld_handler as *const () as u64); // SIGCHLD → reap children

    // Parse services from config
    let data = read_whole_file("/etc/init.toml").unwrap_or_default();
    let (_, raw_services) = parse_config(&data);

    // Start services
    let mut services: BTreeMap<String, Service> = BTreeMap::new();
    for (name, exec, respawn) in raw_services {
        let svc = Service {
            name: name.clone(),
            exec,
            respawn,
            pid: 0,
            running: false,
        };
        if let Some(pid) = start_service(&svc) {
            services.insert(name, Service { pid, running: true, ..svc });
        }
    }

    // Main loop: reap children, respawn as needed
    loop {
        let r = unsafe { syscall2(61, 0, 0x00000001) }; // wait4(-1, WNOHANG)
        if r > 0 {
            // Find which service exited
            let exited_pid = r as u64;
            let mut dead_names: Vec<String> = Vec::new();
            for (name, svc) in services.iter_mut() {
                if svc.running && svc.pid == exited_pid {
                    svc.running = false;
                    dead_names.push(name.clone());
                    write_all(1, alloc::format!("[init] {} (pid {}) exited\n", name, exited_pid).as_bytes()).ok();
                }
            }
            // Respawn
            for name in dead_names {
                if let Some(svc) = services.remove(&name) {
                    if svc.respawn {
                        write_all(1, alloc::format!("[init] respawning {}...\n", name).as_bytes()).ok();
                        if let Some(pid) = start_service(&svc) {
                            services.insert(name.clone(), Service { pid, running: true, ..svc });
                        }
                    }
                }
            }
        } else if r < 0 && r != -10 { // -ECHILD = no children
            // wait4 returned error other than ECHILD
        }

        if SHUTDOWN.load(Ordering::SeqCst) {
            break;
        }

        // Yield to avoid busy-waiting
        unsafe { syscall1(35, 50_000_000); } // nanosleep 50ms
    }

    shutdown_all(&mut services, REBOOT.load(Ordering::SeqCst));
}

sarga_main!(user_main);
