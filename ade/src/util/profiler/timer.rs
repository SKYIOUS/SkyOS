#[derive(Clone, Copy)]
pub(crate) struct ProfilerTimer {
    pub start_tick: u64,
    pub accumulated: u64,
    pub count: u32,
}

impl ProfilerTimer {
    pub fn new() -> Self {
        ProfilerTimer {
            start_tick: 0,
            accumulated: 0,
            count: 0,
        }
    }
    pub fn start(&mut self, tick: u64) {
        self.start_tick = tick;
    }
    pub fn stop(&mut self, tick: u64) {
        self.accumulated += tick.wrapping_sub(self.start_tick);
        self.count += 1;
    }
    pub fn avg(&self) -> u64 {
        if self.count > 0 {
            self.accumulated / self.count as u64
        } else {
            0
        }
    }
    pub fn reset(&mut self) {
        self.accumulated = 0;
        self.count = 0;
    }
}
