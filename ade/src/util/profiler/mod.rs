pub(crate) mod counter;
pub(crate) mod memory;
pub(crate) mod metrics;
pub(crate) mod timer;
pub(crate) mod trace;

pub(crate) use counter::ProfilerCounter;
pub(crate) use memory::MemoryStats;
pub(crate) use metrics::MetricsSnapshot;
pub(crate) use timer::ProfilerTimer;
pub(crate) use trace::TraceBuffer;

#[allow(dead_code)]
pub(crate) struct Profiler {
    pub frame_timer: ProfilerTimer,
    pub flush_timer: ProfilerTimer,
    pub event_counter: ProfilerCounter,
    pub draw_call_counter: ProfilerCounter,
    pub dirty_region_counter: ProfilerCounter,
    pub trace: TraceBuffer,
    pub memory_stats: MemoryStats,
}

#[allow(dead_code)]
impl Profiler {
    pub fn new() -> Self {
        Profiler {
            frame_timer: ProfilerTimer::new(),
            flush_timer: ProfilerTimer::new(),
            event_counter: ProfilerCounter::new(),
            draw_call_counter: ProfilerCounter::new(),
            dirty_region_counter: ProfilerCounter::new(),
            trace: TraceBuffer::new(),
            memory_stats: MemoryStats::new(),
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            frame_time_avg: self.frame_timer.avg(),
            draw_calls: self.draw_call_counter.value,
            dirty_regions: self.dirty_region_counter.value,
            flush_time_avg: self.flush_timer.avg(),
            event_dispatch_count: self.event_counter.value,
            heap_usage: self.memory_stats.heap_usage,
            peak_heap: self.memory_stats.peak_heap,
            alloc_count: self.memory_stats.alloc_count,
            free_count: self.memory_stats.free_count,
        }
    }

    pub fn reset(&mut self) {
        self.frame_timer.reset();
        self.flush_timer.reset();
        self.event_counter.reset();
        self.draw_call_counter.reset();
        self.dirty_region_counter.reset();
    }

    pub fn trace(&mut self, tick: u64, event: &'static str, data: u64) {
        self.trace.push(tick, event, data);
    }
}
