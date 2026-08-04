//! File watching — poll directories for new/deleted/renamed files.
#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

pub(crate) struct FileWatcher {
    watched: Vec<WatchedDir>,
}

struct WatchedDir {
    path: String,
    known: BTreeMap<String, bool>, // name → is_dir
    changed: bool,
}

impl FileWatcher {
    pub fn new() -> Self {
        FileWatcher {
            watched: Vec::new(),
        }
    }

    pub fn watch(&mut self, path: &str) {
        if self.watched.iter().any(|w| w.path == path) {
            return;
        }
        let known = Self::scan(path);
        self.watched.push(WatchedDir {
            path: String::from(path),
            known,
            changed: false,
        });
    }

    pub fn poll(&mut self) {
        for w in &mut self.watched {
            let current = Self::scan(&w.path);
            w.changed = current != w.known;
            w.known = current;
        }
    }

    pub fn has_changed(&self) -> bool {
        self.watched.iter().any(|w| w.changed)
    }

    pub fn changes(&mut self) -> Vec<String> {
        let mut result = Vec::new();
        for w in &mut self.watched {
            if w.changed {
                result.push(w.path.clone());
                w.changed = false;
            }
        }
        result
    }

    fn scan(path: &str) -> BTreeMap<String, bool> {
        let mut map = BTreeMap::new();
        let fd = match libsarga::io::open(path, 0) {
            Ok(f) => f,
            _ => return map,
        };
        let mut buf = [0u8; 4096];
        loop {
            let n = libsarga::io::read(fd, &mut buf).unwrap_or(0);
            if n == 0 {
                break;
            }
            let mut off = 0usize;
            while off + 19 <= n as usize {
                let ino = u64::from_ne_bytes(buf[off..off + 8].try_into().unwrap_or([0; 8]));
                if ino == 0 {
                    break;
                }
                let _ent_len =
                    u64::from_ne_bytes(buf[off + 8..off + 16].try_into().unwrap_or([0; 8]))
                        as usize;
                let kind = buf[off + 16];
                off += 17;
                let name_end = off + buf[off..].iter().position(|&b| b == 0).unwrap_or(0);
                let name = core::str::from_utf8(&buf[off..name_end]).unwrap_or("");
                if !name.is_empty() && name != "." {
                    map.insert(String::from(name), kind == 1);
                }
                off = name_end + 1;
                if off > n as usize {
                    break;
                }
            }
        }
        let _ = libsarga::io::close(fd);
        map
    }
}
