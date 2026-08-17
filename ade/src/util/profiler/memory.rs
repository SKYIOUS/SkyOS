pub(crate) struct MemoryStats {
    pub heap_usage: usize,
}

impl MemoryStats {
    pub fn new() -> Self {
        MemoryStats { heap_usage: 0 }
    }
}
