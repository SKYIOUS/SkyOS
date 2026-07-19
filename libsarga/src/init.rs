use crate::process;
use crate::thread;

pub struct Service {
    pub name: &'static str,
    pub command: &'static str,
    pub depends: &'static [&'static str],
    pub respawn: bool,
    pub respawn_delay_ms: u64,
}

pub struct InitManager {
    services: &'static [Service],
}

impl InitManager {
    pub const fn new(services: &'static [Service]) -> Self {
        InitManager { services }
    }

    pub fn start_all(&self) -> ! {
        let n = self.services.len();
        let mut started = alloc::vec![false; n];
        let mut pids = alloc::vec![0u64; n];

        loop {
            let mut progress = false;
            for (i, svc) in self.services.iter().enumerate() {
                if started[i] {
                    continue;
                }
                let deps_met = svc.depends.iter().all(|dep| {
                    self.services
                        .iter()
                        .enumerate()
                        .any(|(j, s)| s.name == *dep && started[j])
                });
                if !deps_met {
                    continue;
                }
                progress = true;
                match process::spawn(svc.command) {
                    Ok(pid) => {
                        started[i] = true;
                        pids[i] = pid;
                    }
                    Err(_) => {}
                }
            }
            if !progress {
                break;
            }
        }

        loop {
            for (i, svc) in self.services.iter().enumerate() {
                if !svc.respawn || !started[i] {
                    continue;
                }
                match process::waitpid(pids[i] as i64, 0) {
                    Ok((_, _)) => match process::spawn(svc.command) {
                        Ok(pid) => pids[i] = pid,
                        Err(_) => {}
                    },
                    Err(_) => {}
                }
            }
            thread::sleep_ms(1000);
        }
    }
}
