//! Power manager — idle tracking.

pub(crate) struct PowerManager {
    idle_ticks: u64,
    last_activity_tick: u64,
}

impl PowerManager {
    pub fn new() -> Self {
        PowerManager {
            idle_ticks: 0,
            last_activity_tick: 0,
        }
    }

    pub fn tick(&mut self, current_tick: u64) {
        if current_tick > self.last_activity_tick {
            self.idle_ticks = current_tick - self.last_activity_tick;
        }
    }
}
