//! Session tracking — uptime, top-level lifecycle.

#[allow(dead_code)]
pub(crate) struct SessionService {
    pub started: u64,
}

#[allow(dead_code)]
impl SessionService {
    pub fn new() -> Self {
        SessionService { started: 0 }
    }

    pub fn uptime(&self, ticks: u64) -> u64 {
        ticks.saturating_sub(self.started)
    }
}
