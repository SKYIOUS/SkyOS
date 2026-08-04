pub(crate) struct MemoryStats {
    pub heap_usage: usize,
    pub peak_heap: usize,
    pub alloc_count: u64,
    pub free_count: u64,
    #[allow(dead_code)] // reserved per-subsystem memory accounting
    pub window_memory: usize,
    #[allow(dead_code)] // reserved per-subsystem memory accounting
    pub notification_memory: usize,
    #[allow(dead_code)] // reserved per-subsystem memory accounting
    pub clipboard_memory: usize,
}

impl MemoryStats {
    pub fn new() -> Self {
        MemoryStats {
            heap_usage: 0,
            peak_heap: 0,
            alloc_count: 0,
            free_count: 0,
            window_memory: 0,
            notification_memory: 0,
            clipboard_memory: 0,
        }
    }

    #[allow(dead_code)] // profiler API
    pub fn record_alloc(&mut self, size: usize) {
        self.heap_usage += size;
        self.peak_heap = core::cmp::max(self.peak_heap, self.heap_usage);
        self.alloc_count += 1;
    }

    #[allow(dead_code)] // profiler API
    pub fn record_free(&mut self, size: usize) {
        self.heap_usage = self.heap_usage.saturating_sub(size);
        self.free_count += 1;
    }
}
