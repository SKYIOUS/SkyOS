pub(crate) mod counter;
pub(crate) mod memory;
pub(crate) mod metrics;
pub(crate) mod timer;

pub(crate) use counter::ProfilerCounter;
pub(crate) use memory::MemoryStats;
pub(crate) use metrics::MetricsSnapshot;
pub(crate) use timer::ProfilerTimer;

pub(crate) struct Profiler {
    pub frame_timer: ProfilerTimer,
    pub event_counter: ProfilerCounter,
    pub dirty_region_counter: ProfilerCounter,
    pub memory_stats: MemoryStats,
}

impl Profiler {
    pub fn new() -> Self {
        Profiler {
            frame_timer: ProfilerTimer::new(),
            event_counter: ProfilerCounter::new(),
            dirty_region_counter: ProfilerCounter::new(),
            memory_stats: MemoryStats::new(),
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            frame_time_avg: self.frame_timer.avg(),
            dirty_regions: self.dirty_region_counter.value,
            event_dispatch_count: self.event_counter.value,
            heap_usage: self.memory_stats.heap_usage,
        }
    }
}
