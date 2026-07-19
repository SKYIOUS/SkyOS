//! Clock formatting — tick-to-time-string conversion with caching.

pub struct ClockCache {
    hrs: u64,
    mins: u64,
    formatted: alloc::string::String,
}

impl ClockCache {
    pub fn new() -> Self {
        Self {
            hrs: u64::MAX,
            mins: u64::MAX,
            formatted: alloc::string::String::new(),
        }
    }
}

pub fn format_time(ticks: u64, cache: &mut ClockCache) -> &str {
    let secs = ticks / 60;
    let hrs = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    if hrs != cache.hrs || mins != cache.mins {
        cache.hrs = hrs;
        cache.mins = mins;
        cache.formatted = alloc::format!("{:02}:{:02}", hrs, mins);
    }
    &cache.formatted
}
