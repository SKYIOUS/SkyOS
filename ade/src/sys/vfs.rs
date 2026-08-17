//! Virtual filesystem integration — directory listing.

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub(crate) struct VfsEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
}

pub(crate) struct VfsContext;

impl VfsContext {
    pub fn list_dir_static(path: &str) -> Vec<VfsEntry> {
        list_dir(path)
    }
}

fn list_dir(path: &str) -> Vec<VfsEntry> {
    let mut entries = Vec::new();
    let fd = match libsarga::io::open(path, 0) {
        Ok(f) => f,
        _ => return entries,
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
                u64::from_ne_bytes(buf[off + 8..off + 16].try_into().unwrap_or([0; 8])) as usize;
            let ty = buf[off + 16];
            off += 17;
            let name_end = off + buf[off..].iter().position(|&b| b == 0).unwrap_or(0);
            let name = core::str::from_utf8(&buf[off..name_end]).unwrap_or("");
            if name != "." && !name.is_empty() {
                let full = if path.ends_with('/') {
                    alloc::format!("{}{}", path, name)
                } else {
                    alloc::format!("{}/{}", path, name)
                };
                let mut size = 0u64;
                let mut modified = 0u64;
                if let Ok(stat) = libsarga::fs::stat(&full) {
                    size = stat.size as u64;
                    modified = stat.mtime;
                }
                entries.push(VfsEntry {
                    name: String::from(name),
                    path: full,
                    is_dir: ty == 1,
                    size,
                    modified,
                });
            }
            off = name_end + 1;
            if off > n as usize {
                break;
            }
        }
    }
    let _ = libsarga::io::close(fd);
    entries
}
