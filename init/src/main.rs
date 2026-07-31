#![no_std]
#![no_main]
extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use libsarga::errno::Error;
use libsarga::io;
use libsarga::process;
use libsarga::sarga_main;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

const MAX_RESPAWNS: u32 = 5;
// ponytail: single shared backoff across services — per-service backoff if
// one bad service should not stall the others.

struct Service {
    name: String,
    exec: String,
    respawn: bool,
    pid: Option<u64>,
    crashes: u32,
}

impl Service {
    fn spawn(&mut self) -> Result<(), Error> {
        let _ = io::write_all(1, b"[init] starting service: ");
        let _ = io::write_all(1, self.name.as_bytes());
        let _ = io::write_all(1, b"\n");

        match process::fork() {
            Ok(0) => {
                // Child
                if let Err(_e) = process::execve(&self.exec, &[], &[]) {
                    let msg = alloc::format!("[init] exec failed for {}: {}\n", self.name, _e);
                    let _ = io::write_all(1, msg.as_bytes());
                    process::exit(1);
                }
                process::exit(0);
            }
            Ok(pid) => {
                self.pid = Some(pid);
                Ok(())
            }
            Err(e) => {
                let _ = io::write_all(1, b"[init] fork failed for ");
                let _ = io::write_all(1, self.name.as_bytes());
                let _ = io::write_all(1, b"\n");
                Err(e)
            }
        }
    }
}

fn user_main() -> i32 {
    let _ = io::write_all(1, b"[init] SARGA init starting\n");
    let _ = io::write_all(1, b"Userland init running\n");

    // Mount essential filesystems
    let _ = io::mkdir("/tmp", 0o777);
    let _ = io::mkdir("/dev", 0o755);
    let _ = io::mkdir("/ctl", 0o755);
    if let Err(_) = io::mount("none", "/tmp", "tmpfs", 0) {
        let _ = io::write_all(1, b"[init] WARN: failed to mount /tmp\n");
    }
    if let Err(_) = io::mount("none", "/dev", "devfs", 0) {
        let _ = io::write_all(1, b"[init] WARN: failed to mount /dev\n");
    }
    if let Err(_) = io::mount("none", "/ctl", "ctlfs", 0) {
        let _ = io::write_all(1, b"[init] WARN: failed to mount /ctl\n");
    }

    let mut services = Vec::new();
    services.push(Service {
        name: "login-manager".to_string(),
        exec: "/bin/login-manager".to_string(),
        respawn: true,
        pid: None,
        crashes: 0,
    });
    services.push(Service {
        name: "svc".to_string(),
        exec: "/bin/svc".to_string(),
        respawn: true,
        pid: None,
        crashes: 0,
    });

    for svc in &mut services {
        let _ = svc.spawn();
    }

    loop {
        if SHUTDOWN.load(Ordering::Acquire) {
            break;
        }

        // Wait for any child process to exit (-1 means any child)
        match process::waitpid(-1, 0) {
            Ok((pid, _status)) => {
                let mut found = false;
                for svc in &mut services {
                    if svc.pid == Some(pid) {
                        let _ = io::write_all(1, b"[init] service ");
                        let _ = io::write_all(1, svc.name.as_bytes());
                        let _ = io::write_all(1, b" exited\n");

                        svc.pid = None;
                        if svc.respawn {
                            svc.crashes += 1;
                            if svc.crashes > MAX_RESPAWNS {
                                let _ = io::write_all(1, b"[init] giving up on ");
                                let _ = io::write_all(1, svc.name.as_bytes());
                                let _ = io::write_all(1, b" after too many crashes\n");
                                svc.respawn = false;
                            } else {
                                let _ = io::nanosleep(500_000_000);
                                let _ = svc.spawn();
                            }
                        }
                        found = true;
                        break;
                    }
                }
                if !found {
                    // Orphaned process exited
                }
            }
            Err(_) => {
                let _ = io::nanosleep(100_000_000);
            }
        }
    }

    let _ = io::write_all(1, b"[init] shutting down\n");
    0
}

sarga_main!(user_main);
