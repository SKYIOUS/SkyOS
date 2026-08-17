#![no_std]
#![no_main]
extern crate alloc;

use alloc::string::{String, ToString};
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
    /// Extra argv entries passed to execve after argv[0] (the path).
    /// Empty for stock services; the force-vahid-fail feature adds
    /// "--force-fail" so CI can drive vahid's fatal /dev path.
    args: alloc::vec::Vec<alloc::string::String>,
}

impl Service {
    fn spawn(&mut self) -> Result<(), Error> {
        let _ = io::write_all(1, b"[init] starting service: ");
        let _ = io::write_all(1, self.name.as_bytes());
        let _ = io::write_all(1, b"\n");

        match process::fork() {
            Ok(0) => {
                // Child: pass argv[0] (the path) plus any service args
                // so services see their own name and flags; empty argv
                // made getopt/argv scans misbehave.
                let path = self.exec.as_str();
                let mut argv: alloc::vec::Vec<&str> = alloc::vec![path];
                argv.extend(self.args.iter().map(|a| a.as_str()));
                if let Err(_e) = process::execve(path, &argv, &[]) {
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
    if io::mount("none", "/tmp", "tmpfs", 0).is_err() {
        let _ = io::write_all(1, b"[init] WARN: failed to mount /tmp\n");
    }
    if io::mount("none", "/dev", "devfs", 0).is_err() {
        let _ = io::write_all(1, b"[init] WARN: failed to mount /dev\n");
    }
    if io::mount("none", "/ctl", "ctlfs", 0).is_err() {
        let _ = io::write_all(1, b"[init] WARN: failed to mount /ctl\n");
    }

    // vahid first: device manager (PCI scan + /dev node creation) so the
    // console/tty nodes exist before login-manager and the getty run.
    // Then login-manager (GUI), svc (service daemon), and the console
    // getty (/bin/login on the inherited console fds - Phase A of the
    // session-lifecycle plan, CI drives root/skyos through it).
    let mut services = alloc::vec![
        Service {
            name: "vahid".to_string(),
            exec: "/bin/vahid".to_string(),
            respawn: true,
            pid: None,
            crashes: 0,
            // Test-only: the force-vahid-fail feature makes init spawn
            // vahid with --force-fail, driving its FATAL /dev path so
            // "[init] giving up on vahid" gets a real QEMU run.
            args: if cfg!(feature = "force-vahid-fail") {
                alloc::vec!["--force-fail".to_string()]
            } else {
                alloc::vec![]
            },
        },
        Service {
            name: "login-manager".to_string(),
            exec: "/bin/login-manager".to_string(),
            respawn: true,
            pid: None,
            crashes: 0,
            args: alloc::vec![],
        },
        Service {
            name: "svc".to_string(),
            exec: "/bin/svc".to_string(),
            // On the fail-vahid build, svc is a ONE-SHOT: its Usage exit is
            // far faster than vahid's forced /dev path, so respawning svc
            // would race vahid for MAX_RESPAWNS and the harness's
            // boundedness wait could fire on svc first, hiding the
            // 'giving up on vahid' the dedicated boot must prove. vahid
            // then is the sole bounded non-zero service, deterministically.
            respawn: !cfg!(feature = "force-vahid-fail"),
            pid: None,
            crashes: 0,
            args: alloc::vec![],
        },
        Service {
            name: "getty".to_string(),
            exec: "/bin/login".to_string(),
            respawn: true,
            pid: None,
            crashes: 0,
            args: alloc::vec![],
        },
    ];

    for svc in &mut services {
        let _ = svc.spawn();
    }

    loop {
        if SHUTDOWN.load(Ordering::Acquire) {
            break;
        }

        // Wait for any child process to exit (-1 means any child)
        match process::waitpid(-1, 0) {
            Ok((pid, status)) => {
                let mut found = false;
                for svc in &mut services {
                    if svc.pid == Some(pid) {
                        let _ = io::write_all(1, b"[init] service ");
                        let _ = io::write_all(1, svc.name.as_bytes());
                        let _ = io::write_all(1, b" exited\n");

                        svc.pid = None;
                        // Clean exit (status == 0) means the service ran its
                        // course — reset the crash counter so a single bad
                        // streak doesn't permanently kill a service.
                        if status == 0 {
                            svc.crashes = 0;
                        }
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
