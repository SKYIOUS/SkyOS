//! Power manager — idle tracking, battery status, power state requests.

pub(crate) struct PowerManager {
    pub(crate) battery_available: bool,
    pub(crate) battery_percentage: u8,
    pub(crate) ac_connected: bool,
    pub(crate) suspend_requested: bool,
    #[allow(dead_code)] // power-state flags, read by power-off path when wired
    pub(crate) shutdown_requested: bool,
    #[allow(dead_code)] // power-state flags, read by power-off path when wired
    pub(crate) restart_requested: bool,
    #[allow(dead_code)] // power-state flags, read by power-off path when wired
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

    #[allow(dead_code)] // activity tracking API, idle sleep not wired yet
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

    #[allow(dead_code)] // power-off requests, no kernel hook yet
    pub fn request_shutdown(&mut self) {
        self.shutdown_requested = true;
    }

    #[allow(dead_code)] // power-off requests, no kernel hook yet
    pub fn request_restart(&mut self) {
        self.restart_requested = true;
    }

    #[allow(dead_code)] // power-off requests, no kernel hook yet
    pub fn request_sleep(&mut self) {
        self.sleep_requested = true;
    }

    #[allow(dead_code)] // battery polling API, no battery backend yet
    pub fn set_battery(&mut self, available: bool, percentage: u8, ac: bool) {
        self.battery_available = available;
        self.battery_percentage = percentage;
        self.ac_connected = ac;
    }
}
