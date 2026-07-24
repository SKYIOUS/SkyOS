//! Power manager — idle tracking, battery status, power state requests.

pub(crate) struct PowerManager {
    pub(crate) battery_available: bool,
    pub(crate) battery_percentage: u8,
    pub(crate) ac_connected: bool,
    pub(crate) suspend_requested: bool,
    pub(crate) shutdown_requested: bool,
    pub(crate) restart_requested: bool,
    pub(crate) sleep_requested: bool,
    idle_ticks: u64,
    last_activity_tick: u64,
}

impl PowerManager {
    pub fn new() -> Self {
        PowerManager {
            battery_available: false,
            battery_percentage: 100,
            ac_connected: true,
            suspend_requested: false,
            shutdown_requested: false,
            restart_requested: false,
            sleep_requested: false,
            idle_ticks: 0,
            last_activity_tick: 0,
        }
    }

    pub fn mark_activity(&mut self, tick: u64) {
        self.last_activity_tick = tick;
        self.idle_ticks = 0;
    }

    pub fn tick(&mut self, current_tick: u64) {
        if current_tick > self.last_activity_tick {
            self.idle_ticks = current_tick - self.last_activity_tick;
        }
    }

    pub fn request_suspend(&mut self) {
        self.suspend_requested = true;
    }

    pub fn request_shutdown(&mut self) {
        self.shutdown_requested = true;
    }

    pub fn request_restart(&mut self) {
        self.restart_requested = true;
    }

    pub fn request_sleep(&mut self) {
        self.sleep_requested = true;
    }

    pub fn set_battery(&mut self, available: bool, percentage: u8, ac: bool) {
        self.battery_available = available;
        self.battery_percentage = percentage;
        self.ac_connected = ac;
    }
}
