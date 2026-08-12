pub(crate) struct Logger;

impl Logger {
    pub fn new() -> Self {
        Logger
    }

    pub fn info(&mut self, _tick: u64, _msg: &'static str) {
        // Log sink not wired yet; kept for call-site stability.
    }
}
