#[derive(Clone, Copy)]
pub(crate) struct MetricsSnapshot {
    pub frame_time_avg: u64,
    pub draw_calls: u64,
    pub dirty_regions: u64,
    pub flush_time_avg: u64,
    pub event_dispatch_count: u64,
    pub heap_usage: usize,
    pub peak_heap: usize,
    pub alloc_count: u64,
    pub free_count: u64,
}
