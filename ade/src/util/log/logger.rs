use crate::util::log::levels::LogLevel;
use crate::util::log::ring_buffer::LogRingBuffer;

pub(crate) struct Logger {
    pub buffer: LogRingBuffer,
    pub level_filter: LogLevel,
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            buffer: LogRingBuffer::new(),
            level_filter: LogLevel::Info,
        }
    }

    pub fn log(&mut self, tick: u64, level: LogLevel, msg: &'static str) {
        if (level as u8) >= (self.level_filter as u8) {
            self.buffer.push(tick, level, msg);
        }
    }

    #[allow(dead_code)] // logger convenience shorthand
    pub fn trace(&mut self, tick: u64, msg: &'static str) {
        self.log(tick, LogLevel::Trace, msg);
    }

    pub fn info(&mut self, tick: u64, msg: &'static str) {
        self.log(tick, LogLevel::Info, msg);
    }

    #[allow(dead_code)] // logger convenience shorthand
    pub fn warn(&mut self, tick: u64, msg: &'static str) {
        self.log(tick, LogLevel::Warn, msg);
    }

    #[allow(dead_code)] // logger convenience shorthand
    pub fn error(&mut self, tick: u64, msg: &'static str) {
        self.log(tick, LogLevel::Error, msg);
    }
}
