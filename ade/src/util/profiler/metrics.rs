#[derive(Clone, Copy)]
pub(crate) struct MetricsSnapshot {
    pub frame_time_avg: u64,
    #[allow(dead_code)] // metrics snapshot surface, fully populated for the debug overlay
    pub draw_calls: u64,
    pub dirty_regions: u64,
    #[allow(dead_code)] // metrics snapshot surface, fully populated for the debug overlay
    pub flush_time_avg: u64,
    pub event_dispatch_count: u64,
    pub heap_usage: usize,
    #[allow(dead_code)] // metrics snapshot surface, fully populated for the debug overlay
    pub peak_heap: usize,
    #[allow(dead_code)] // metrics snapshot surface, fully populated for the debug overlay
    pub alloc_count: u64,
    #[allow(dead_code)] // metrics snapshot surface, fully populated for the debug overlay
    pub free_count: u64,
}
