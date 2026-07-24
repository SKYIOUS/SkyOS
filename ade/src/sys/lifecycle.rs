//! Application lifecycle — state machine, restart policy, crash detection.

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum AppState {
    Starting,
    Running,
    Suspended,
    Hidden,
    Terminated,
    Crashed,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub(crate) enum RestartPolicy {
    Never,
    Always,
    OnCrash,
}

pub(crate) struct AppLifecycle {
    pub pid: u64,
    pub state: AppState,
    #[allow(dead_code)]
    pub app_idx: usize,
    #[allow(dead_code)]
    pub restart: RestartPolicy,
    #[allow(dead_code)]
    pub crash_count: u32,
}

pub(crate) struct LifecycleManager {
    pub procs: Vec<AppLifecycle>,
}

impl LifecycleManager {
    pub fn new() -> Self {
        LifecycleManager { procs: Vec::new() }
    }

    pub fn register(&mut self, pid: u64, app_idx: usize) {
        self.procs.push(AppLifecycle {
            pid,
            state: AppState::Starting,
            app_idx,
            restart: RestartPolicy::OnCrash,
            crash_count: 0,
        });
    }

    #[allow(dead_code)]
    pub fn mark_running(&mut self, pid: u64) {
        for p in &mut self.procs {
            if p.pid == pid && p.state == AppState::Starting {
                p.state = AppState::Running;
                return;
            }
        }
    }

    pub fn mark_terminated(&mut self, pid: u64) {
        for p in &mut self.procs {
            if p.pid == pid {
                p.state = AppState::Terminated;
                return;
            }
        }
    }

    #[allow(dead_code)]
    pub fn mark_crashed(&mut self, pid: u64) {
        for p in &mut self.procs {
            if p.pid == pid {
                p.state = AppState::Crashed;
                p.crash_count += 1;
                return;
            }
        }
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, pid: u64) {
        self.procs.retain(|p| p.pid != pid);
    }

    #[allow(dead_code)]
    pub fn get(&self, pid: u64) -> Option<&AppLifecycle> {
        self.procs.iter().find(|p| p.pid == pid)
    }

    #[allow(dead_code)]
    pub fn get_mut(&mut self, pid: u64) -> Option<&mut AppLifecycle> {
        self.procs.iter_mut().find(|p| p.pid == pid)
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.procs.clear();
    }
}
