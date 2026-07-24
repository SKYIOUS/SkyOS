#[allow(dead_code)]
pub(crate) struct CrashDiagnostics {
    pub panic_reason: Option<&'static str>,
    pub last_events: [&'static str; 16],
    pub last_event_index: usize,
    pub last_event_count: usize,
    pub last_focused_window: Option<u32>,
    pub last_notification: Option<&'static str>,
    pub uptime_at_crash: u64,
    pub restart_count: u32,
}

#[allow(dead_code)]
impl CrashDiagnostics {
    pub fn new() -> Self {
        CrashDiagnostics {
            panic_reason: None,
            last_events: [""; 16],
            last_event_index: 0,
            last_event_count: 0,
            last_focused_window: None,
            last_notification: None,
            uptime_at_crash: 0,
            restart_count: 0,
        }
    }

    pub fn record_event(&mut self, event: &'static str) {
        self.last_events[self.last_event_index] = event;
        self.last_event_index = (self.last_event_index + 1) % 16;
        if self.last_event_count < 16 {
            self.last_event_count += 1;
        }
    }

    pub fn record_panic(&mut self, reason: &'static str, uptime: u64) {
        self.panic_reason = Some(reason);
        self.uptime_at_crash = uptime;
        self.restart_count += 1;
    }
}
