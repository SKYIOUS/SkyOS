#[derive(Clone, Copy)]
pub(crate) struct TraceEntry {
    #[allow(dead_code)] // trace entry data surface, read via future trace UI
    pub tick: u64,
    #[allow(dead_code)] // trace entry data surface, read via future trace UI
    pub event: &'static str,
    #[allow(dead_code)] // trace entry data surface, read via future trace UI
    pub data: u64,
}

pub(crate) struct TraceBuffer {
    pub entries: [TraceEntry; 256],
    pub index: usize,
    pub count: usize,
}

impl TraceBuffer {
    pub fn new() -> Self {
        TraceBuffer {
            entries: [TraceEntry {
                tick: 0,
                event: "",
                data: 0,
            }; 256],
            index: 0,
            count: 0,
        }
    }

    pub fn push(&mut self, tick: u64, event: &'static str, data: u64) {
        self.entries[self.index] = TraceEntry { tick, event, data };
        self.index = (self.index + 1) % 256;
        if self.count < 256 {
            self.count += 1;
        }
    }

    #[allow(dead_code)] // trace API reader
    pub fn iter(&self) -> core::slice::Iter<'_, TraceEntry> {
        self.entries[..self.count].iter()
    }
}
