#![no_std]
#![no_main]
extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::fs;
use libsarga::io::{self, close, getdents64, open, stat, Stat};
use libsarga::sarga_main;

const SKIP_PREFIXES: &[&str] = &[
    "/dev/",
    "/proc/",
    "/sys/",
    "/tmp/",
    "/var/cache/",
    "/var/spool/",
    "/var/log/",
    "/mnt/",
];

fn is_skipped(path: &str) -> bool {
    SKIP_PREFIXES
        .iter()
        .any(|p| path.starts_with(p) || path == p.trim_end_matches('/'))
}

fn walk_dir(path: &str, entries: &mut Vec<(String, String, u64, bool)>, depth: usize) {
    if depth > 6 {
        return;
    }
    if path.len() > 256 || is_skipped(path) {
        return;
    }
    let fd = match open(path, 0) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut buf = [0u8; 4096];
    let n = match getdents64(fd, &mut buf) {
        Ok(n) => n,
        Err(_) => {
            let _ = close(fd);
            return;
        }
    };
    let _ = close(fd);

    let mut off = 0;
    while off < n {
        if off + 19 > n {
            break;
        }
        let reclen =
            u16::from_le_bytes(buf[off + 16..off + 18].try_into().unwrap_or([0; 2])) as usize;
        let d_type = buf[off + 18];
        if reclen < 19 || off + reclen > n {
            break;
        }
        let name_end = buf[off + 19..off + reclen]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(reclen - 19);
        let name = core::str::from_utf8(&buf[off + 19..off + 19 + name_end]).unwrap_or("");
        if name.is_empty() || name == "." || name == ".." {
            off += reclen;
            continue;
        }

        let full = if path == "/" {
            format!("/{}", name)
        } else {
            format!("{}/{}", path, name)
        };

        if is_skipped(&full) {
            off += reclen;
            continue;
        }

        let is_dir = d_type == 4;
        let size = stat(&full).map(|s: Stat| s.size).unwrap_or(0);
        entries.push((name.to_string(), full.clone(), size, is_dir));

        if is_dir && depth < 6 {
            walk_dir(&full, entries, depth + 1);
        }
        off += reclen;
    }
}

fn build_index() -> Vec<(String, String, u64, bool)> {
    let mut entries = Vec::new();
    walk_dir("/", &mut entries, 0);
    entries
}

fn write_index(entries: &[(String, String, u64, bool)]) {
    let mut out = String::new();
    for (name, path, size, is_dir) in entries {
        let escaped_path = path.replace('|', "_");
        out.push_str(&format!(
            "{}|{}|{}|{}\n",
            name, escaped_path, size, *is_dir as u8
        ));
    }
    // Atomic write: write to temp then rename
    let _ = fs::write_file("/tmp/search.idx.tmp", &out);
    let _ = libsarga::io::rename("/tmp/search.idx.tmp", "/tmp/search.idx");
}

fn user_main() -> i32 {
    io::print_str("[searchd] starting\n");
    let mut cycle = 0u64;

    loop {
        if cycle == 0 || cycle.is_multiple_of(12) {
            io::print_str(&format!("[searchd] indexing (cycle {})\n", cycle));
        }
        let entries = build_index();
        write_index(&entries);

        cycle += 1;
        let _ = io::nanosleep(30_000_000_000);
    }
}

sarga_main!(user_main);
