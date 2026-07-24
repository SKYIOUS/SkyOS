use crate::util::log::levels::LogLevel;

#[derive(Clone, Copy)]
pub(crate) struct LogEntry {
    pub tick: u64,
    pub level: LogLevel,
    pub message: &'static str,
}

pub(crate) struct LogRingBuffer {
    pub entries: [LogEntry; 512],
    pub head: usize,
    pub count: usize,
}

impl LogRingBuffer {
    pub fn new() -> Self {
        LogRingBuffer {
            entries: [LogEntry { tick: 0, level: LogLevel::Trace, message: "" }; 512],
            head: 0,
            count: 0,
        }
    }

    pub fn push(&mut self, tick: u64, level: LogLevel, message: &'static str) {
        self.entries[self.head] = LogEntry { tick, level, message };
        self.head = (self.head + 1) % 512;
        if self.count < 512 {
            self.count += 1;
        }
    }

    pub fn iter(&self) -> core::slice::Iter<'_, LogEntry> {
        self.entries[..self.count].iter()
    }
}
