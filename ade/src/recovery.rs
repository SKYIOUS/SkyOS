//! Recovery system — crash logs, safe mode, session recovery.
#![allow(dead_code)]

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

const CRASH_LOG: &str = "/tmp/skyos.crash";

pub(crate) struct RecoverySystem {
    pub safe_mode: bool,
    pub crash_count: u32,
    pub last_session_saved: bool,
}

impl RecoverySystem {
    pub fn new() -> Self {
        let crash_count = Self::read_crash_count();
        let safe_mode = crash_count >= 3;
        RecoverySystem { safe_mode, crash_count, last_session_saved: false }
    }

    pub fn record_crash(&mut self, app: &str, pid: u64) {
        self.crash_count += 1;
        let entry = format!("{}:{}:{}", app, pid, self.crash_count);
        let data = entry.as_bytes();
        if let Ok(fd) = libsarga::io::open(CRASH_LOG, 0x241) {
            let _ = libsarga::io::write(fd, data);
            let _ = libsarga::io::close(fd);
        }
        if self.crash_count >= 3 {
            self.safe_mode = true;
        }
    }

    pub fn restore_session() -> Vec<String> {
        crate::session::SessionManager::load_lines()
    }

    pub fn clear_crash_count(&mut self) {
        self.crash_count = 0;
        let _ = libsarga::posix::unlink(CRASH_LOG);
    }

    fn read_crash_count() -> u32 {
        let fd = match libsarga::io::open(CRASH_LOG, 0) { Ok(f) => f, _ => return 0 };
        let mut buf = [0u8; 64];
        let n = libsarga::io::read(fd, &mut buf).unwrap_or(0);
        let _ = libsarga::io::close(fd);
        if n > 0 {
            let s = core::str::from_utf8(&buf[..n as usize]).unwrap_or("");
            s.split(':').last().and_then(|c| c.trim().parse().ok()).unwrap_or(0)
        } else { 0 }
    }
}
