#![no_std]
#![no_main]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::io::*;
use libsarga::process::geteuid;
use libsarga::args;
use libsarga::sarga_main;

// ── .skp format constants ───────────────────────────────────────────────────

const SKP_MAGIC: &[u8; 8] = b"SKYPKG01";
const DB_BASE: &str = "/var/lib/spkg/db/";

struct SkpHeader {
    #[allow(dead_code)]
    magic: [u8; 8],
    _version: u32,
    _flags: u32,
    manifest_off: u64,
    manifest_size: u64,
    payload_off: u64,
    payload_size: u64,
    checksum: u64,
}

// ── file I/O helpers ────────────────────────────────────────────────────────

fn read_whole(fd: i64, max: usize) -> Result<Vec<u8>, i64> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = read(fd, &mut tmp)?;
        if n == 0 { break; }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() >= max { break; }
    }
    Ok(buf)
}

fn read_whole_path(path: &str) -> Result<Vec<u8>, i64> {
    let c = alloc::format!("{}\0", path);
    let fd = open(&c, 0)?;
    let d = read_whole(fd, usize::MAX);
    close(fd)?;
    d
}

fn write_file(path: &str, data: &[u8], mode: u32) -> Result<(), i64> {
    let parent = path.rfind('/').map(|i| &path[..i]).unwrap_or("");
    if !parent.is_empty() {
        let pc = alloc::format!("{}\0", parent);
        unsafe { libsarga::syscall::syscall3(83, pc.as_ptr() as u64, 0o755, 0); }
    }
    let c = alloc::format!("{}\0", path);
    let fd = open(&c, 0x41)?; // O_WRONLY | O_CREAT
    write_all(fd, data)?;
    if mode != 0 {
        fchmod(fd as u64, mode);
    }
    close(fd)?;
    Ok(())
}

fn ensure_dir(path: &str) {
    let c = alloc::format!("{}\0", path);
    unsafe { libsarga::syscall::syscall3(83, c.as_ptr() as u64, 0o755, 0); }
}

fn delete_file(path: &str) {
    let c = alloc::format!("{}\0", path);
    unsafe { libsarga::syscall::syscall1(87, c.as_ptr() as u64); }
}

fn remove_dir_if_empty(path: &str) {
    let c = alloc::format!("{}\0", path);
    let fd = match open(&c, 0) {
        Ok(f) => f,
        Err(_) => return,
    };
    let mut buf = [0u8; 512];
    let n = read(fd, &mut buf).unwrap_or(0);
    close(fd).ok();
    let entries = count_dir_entries(&buf[..n]);
    if entries <= 2 { // . and ..
        unsafe { libsarga::syscall::syscall3(84, c.as_ptr() as u64, 0, 0); } // SYS_RMDIR
    }
}

fn count_dir_entries(buf: &[u8]) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i + 16 <= buf.len() {
        let reclen = u16::from_le_bytes([buf[i+8], buf[i+9]]) as usize;
        if reclen < 11 || i + reclen > buf.len() { break; }
        count += 1;
        i += reclen;
    }
    count
}

// ── checksum (simple FNV-1a) ────────────────────────────────────────────────

fn fnv1a(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xCBF29CE484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001B3);
    }
    hash
}

// ── .skp read ───────────────────────────────────────────────────────────────

fn read_skp_header(data: &[u8]) -> Option<SkpHeader> {
    if data.len() < 64 { return None; }
    let read_u64 = |off: usize| -> u64 {
        let b = &data[off..off+8];
        u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    };
    let read_u32 = |off: usize| -> u32 {
        let b = &data[off..off+4];
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    };
    let mut magic = [0u8; 8];
    magic.copy_from_slice(&data[0..8]);
    if &magic != SKP_MAGIC { return None; }
    Some(SkpHeader {
        magic,
        _version: read_u32(8),
        _flags: read_u32(12),
        manifest_off: read_u64(16),
        manifest_size: read_u64(24),
        payload_off: read_u64(32),
        payload_size: read_u64(40),
        checksum: read_u64(48),
    })
}

/// Parse manifest as simple key=value lines
fn parse_manifest(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut section = String::new();
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') { continue; }
        if t.starts_with('[') {
            section = t.trim_matches('[').trim_matches(']').trim().to_string();
            continue;
        }
        if let Some(eq) = t.find('=') {
            let key = if section.is_empty() || section == "package" {
                t[..eq].trim().to_string()
            } else {
                alloc::format!("{}.{}", section, t[..eq].trim())
            };
            let value = t[eq+1..].trim().trim_matches('"').to_string();
            map.insert(key, value);
        }
    }
    map
}

/// Read payload entries from .skp data
fn read_payload(data: &[u8], payload_off: u64, payload_size: u64) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pos = payload_off as usize;
    let end = pos + payload_size as usize;
    while pos + 8 <= end {
        let plen = u64::from_le_bytes([
            data[pos], data[pos+1], data[pos+2], data[pos+3],
            data[pos+4], data[pos+5], data[pos+6], data[pos+7],
        ]) as usize;
        pos += 8;
        if pos + plen + 8 > end { break; }
        let path = core::str::from_utf8(&data[pos..pos+plen]).unwrap_or("").to_string();
        pos += plen;
        let dlen = u64::from_le_bytes([
            data[pos], data[pos+1], data[pos+2], data[pos+3],
            data[pos+4], data[pos+5], data[pos+6], data[pos+7],
        ]) as usize;
        pos += 8;
        if pos + dlen > end { break; }
        let content = data[pos..pos+dlen].to_vec();
        pos += dlen;
        files.push((path, content));
    }
    files
}

// ── build .skp from directory ───────────────────────────────────────────────

fn collect_files(dir: &str) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    let cd = alloc::format!("{}\0", dir);
    let fd = match open(&cd, 0) {
        Ok(f) => f,
        Err(_) => return files,
    };
    let mut buf = [0u8; 4096];
    loop {
        let r = unsafe { libsarga::syscall::syscall3(217, fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (r as i64) <= 0 { break; }
        let entries = &buf[..r as usize];
        let mut i = 0;
        while i + 16 <= entries.len() {
            let reclen = u16::from_le_bytes([entries[i+8], entries[i+9]]) as usize;
            let namelen = entries[i+10] as usize;
            if reclen < 11 || i + reclen > entries.len() { break; }
            if namelen > 0 {
                let name = core::str::from_utf8(&entries[i+11..i+11+namelen]).unwrap_or("");
                if name != "." && name != ".." {
                    let full = if dir.ends_with('/') {
                        alloc::format!("{}{}", dir, name)
                    } else {
                        alloc::format!("{}/{}", dir, name)
                    };
                    // Check if it's a file or dir by trying to read
                    let fc = alloc::format!("{}\0", &full);
                    if let Ok(fd2) = open(&fc, 0) {
                        // Could stat, but simple approach: try to read and treat as dir if fails
                        if let Ok(data) = read_whole(fd2, 1024 * 1024) {
                            files.push((full, data));
                        }
                        close(fd2).ok();
                    }
                }
            }
            i += reclen;
        }
    }
    close(fd).ok();
    files
}

fn build_skp(files: &[(String, Vec<u8>)], manifest: &str) -> Vec<u8> {
    let manifest_bytes = manifest.as_bytes();
    let mut payload = Vec::new();
    for (path, data) in files {
        let path_bytes = path.as_bytes();
        let plen = path_bytes.len() as u64;
        payload.extend_from_slice(&plen.to_le_bytes());
        payload.extend_from_slice(path_bytes);
        let dlen = data.len() as u64;
        payload.extend_from_slice(&dlen.to_le_bytes());
        payload.extend_from_slice(data);
    }

    let manifest_off = 64u64;
    let manifest_size = manifest_bytes.len() as u64;
    let payload_off = manifest_off + manifest_size;
    let payload_size = payload.len() as u64;

    let mut skp = Vec::new();
    // Header
    skp.extend_from_slice(SKP_MAGIC);
    skp.extend_from_slice(&1u32.to_le_bytes()); // version
    skp.extend_from_slice(&0u32.to_le_bytes()); // flags
    skp.extend_from_slice(&manifest_off.to_le_bytes());
    skp.extend_from_slice(&manifest_size.to_le_bytes());
    skp.extend_from_slice(&payload_off.to_le_bytes());
    skp.extend_from_slice(&payload_size.to_le_bytes());
    skp.extend_from_slice(&0u64.to_le_bytes()); // placeholder checksum
    skp.extend_from_slice(&[0u8; 8]); // reserved
    // Manifest
    skp.extend_from_slice(manifest_bytes);
    // Payload
    skp.extend_from_slice(&payload);
    // Compute and write checksum (of everything except [48..64])
    let mut checksum_data = Vec::new();
    checksum_data.extend_from_slice(&skp[..48]);
    checksum_data.extend_from_slice(&skp[56..]);
    let chk = fnv1a(&checksum_data);
    skp[48..56].copy_from_slice(&chk.to_le_bytes());
    skp
}

// ── commands ────────────────────────────────────────────────────────────────

const REPO_DIR: &str = "/var/lib/spkg/repo/";

fn is_installed(name: &str) -> bool {
    let p = alloc::format!("{}{}/manifest", DB_BASE, name);
    read_whole_path(&p).is_ok()
}

fn get_installed_version(name: &str) -> Option<String> {
    let p = alloc::format!("{}{}/manifest", DB_BASE, name);
    let data = read_whole_path(&p).ok()?;
    let text = String::from_utf8_lossy(&data);
    let meta = parse_manifest(&text);
    meta.get("version").cloned()
}

/// Resolve and install dependencies for a package manifest.
/// Uses recursion with visited set to handle transitive deps without cycles.
fn ensure_deps(meta: &BTreeMap<String, String>, visiting: &mut Vec<String>) {
    for (key, raw_deps) in meta {
        if key == "deps" || key.ends_with(".deps") {
            for dep in raw_deps.split(',') {
                let dep = dep.trim();
                if dep.is_empty() { continue; }
                // Parse "name" or "name >= version"
                let dep_name = dep.split_whitespace().next().unwrap_or(dep);
                if is_installed(dep_name) {
                    continue;
                }
                if visiting.contains(&dep_name.to_string()) {
                    print_str(&alloc::format!("spkg: warning: circular dependency {}\n", dep_name));
                    continue;
                }
                // Look for the dep in the repo
                let repo_skp = alloc::format!("{}{}.skp", REPO_DIR, dep_name);
                if read_whole_path(&repo_skp).is_err() {
                    // Try alternate paths
                    let alt = alloc::format!("{}{}/{}.skp", REPO_DIR, dep_name, dep_name);
                    if read_whole_path(&alt).is_err() {
                        print_str(&alloc::format!("spkg: missing dependency: {} (not in repo)\n", dep_name));
                        continue;
                    }
                }
                visiting.push(dep_name.to_string());
                print_str(&alloc::format!("spkg: installing dependency: {}...\n", dep_name));
                install_skp_file(&repo_skp, visiting);
                visiting.pop();
            }
        }
    }
}

fn install_skp_file(path: &str, visiting: &mut Vec<String>) {
    let data = match read_whole_path(path) {
        Ok(d) => d,
        Err(e) => { print_str(&alloc::format!("spkg: failed to read {}: {}\n", path, e)); return; }
    };

    let hdr = match read_skp_header(&data) {
        Some(h) => h,
        None => { print_str("spkg: invalid .skp file\n"); return; }
    };

    if hdr.checksum != 0 {
        let mut ck_data = Vec::new();
        ck_data.extend_from_slice(&data[..48]);
        ck_data.extend_from_slice(&data[56..]);
        if fnv1a(&ck_data) != hdr.checksum {
            print_str("spkg: checksum mismatch (corrupt package)\n");
            return;
        }
    }

    let manifest_str = match core::str::from_utf8(&data[hdr.manifest_off as usize..][..hdr.manifest_size as usize]) {
        Ok(s) => s,
        Err(_) => { print_str("spkg: invalid manifest encoding\n"); return; }
    };
    let meta = parse_manifest(manifest_str);
    let pkg_name = meta.get("name").map(|s| s.as_str()).unwrap_or("unknown");

    // Resolve deps first
    ensure_deps(&meta, visiting);

    let files = read_payload(&data, hdr.payload_off, hdr.payload_size);

    for (path, content) in &files {
        if path.starts_with('/') {
            if let Err(e) = write_file(path, content, 0o644) {
                print_str(&alloc::format!("spkg: failed to write {}: {}\n", path, e));
                return;
            }
        }
    }

    let db_dir = alloc::format!("{}{}", DB_BASE, pkg_name);
    ensure_dir(&db_dir);
    let manifest_path = alloc::format!("{}/manifest", db_dir);
    write_file(&manifest_path, manifest_str.as_bytes(), 0o644).ok();

    let installed_files: Vec<String> = files.iter()
        .filter(|(p, _)| p.starts_with('/'))
        .map(|(p, _)| p.clone())
        .collect();
    let files_path = alloc::format!("{}/files", db_dir);
    write_file(&files_path, installed_files.join("\n").as_bytes(), 0o644).ok();

    print_str(&alloc::format!("spkg: installed {} ({} files)\n", pkg_name, files.len()));
}

fn cmd_install(pkg_file: &str) {
    if geteuid() != 0 {
        print_str("spkg: must be root to install packages\n");
        return;
    }
    let mut visiting = Vec::new();
    install_skp_file(pkg_file, &mut visiting);
}

fn cmd_remove(pkg_name: &str) {
    if geteuid() != 0 {
        print_str("spkg: must be root to remove packages\n");
        return;
    }

    let db_dir = alloc::format!("{}{}", DB_BASE, pkg_name);

    // Read file list
    let files_path = alloc::format!("{}/files", db_dir);
    let files_content = match read_whole_path(&files_path) {
        Ok(d) => String::from_utf8_lossy(&d).to_string(),
        Err(_) => { print_str(&alloc::format!("spkg: {} not installed\n", pkg_name)); return; }
    };

    // Delete each file
    let mut count = 0;
    for line in files_content.lines() {
        let path = line.trim();
        if !path.is_empty() && path.starts_with('/') {
            delete_file(path);
            count += 1;
            // Try to clean up empty parent dirs
            let mut parent = path.rfind('/').map(|i| path[..i].to_string());
            while let Some(p) = parent {
                remove_dir_if_empty(&p);
                parent = p.rfind('/').map(|i| p[..i].to_string());
                if parent.as_deref() == Some("/") { break; }
            }
        }
    }

    // Remove DB
    delete_file(&alloc::format!("{}/manifest", db_dir));
    delete_file(&alloc::format!("{}/files", db_dir));
    delete_file(&db_dir);

    print_str(&alloc::format!("spkg: removed {} ({} files)\n", pkg_name, count));
}

fn cmd_list() {
    ensure_dir(DB_BASE);
    let c = alloc::format!("{}\0", DB_BASE);
    let fd = match open(&c, 0) {
        Ok(f) => f,
        Err(_) => { print_str("spkg: no packages installed\n"); return; }
    };
    let mut buf = [0u8; 4096];
    let mut found = false;
    loop {
        let r = unsafe { libsarga::syscall::syscall3(217, fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (r as i64) <= 0 { break; }
        let entries = &buf[..r as usize];
        let mut i = 0;
        while i + 16 <= entries.len() {
            let reclen = u16::from_le_bytes([entries[i+8], entries[i+9]]) as usize;
            let namelen = entries[i+10] as usize;
            if reclen < 11 || i + reclen > entries.len() { break; }
            if namelen > 0 {
                if let Ok(name) = core::str::from_utf8(&entries[i+11..i+11+namelen]) {
                    if name != "." && name != ".." {
                        // Try to read manifest for version
                        let mp = alloc::format!("{}{}/manifest", DB_BASE, name);
                        let ver = read_whole_path(&mp)
                            .map(|d| {
                                let t = String::from_utf8_lossy(&d);
                                parse_manifest(&t).get("version").cloned().unwrap_or_default()
                            })
                            .unwrap_or_default();
                        if !ver.is_empty() {
                            print_str(&alloc::format!("  {} v{}\n", name, ver));
                        } else {
                            print_str(&alloc::format!("  {}\n", name));
                        }
                        found = true;
                    }
                }
            }
            i += reclen;
        }
    }
    close(fd).ok();
    if !found { print_str("spkg: no packages installed\n"); }
}

fn cmd_info(name: &str) {
    let mp = alloc::format!("{}{}/manifest", DB_BASE, name);
    let data = match read_whole_path(&mp) {
        Ok(d) => d,
        Err(_) => { print_str(&alloc::format!("spkg: {} not installed\n", name)); return; }
    };
    let text = String::from_utf8_lossy(&data);
    let meta = parse_manifest(&text);
    let mut keys: Vec<&String> = meta.keys().collect();
    keys.sort();
    for k in keys {
        if let Some(v) = meta.get(k) {
            print_str(&alloc::format!("{} = {}\n", k, v));
        }
    }
    // Also show file count
    let fp = alloc::format!("{}{}/files", DB_BASE, name);
    if let Ok(fd) = read_whole_path(&fp) {
        let count = String::from_utf8_lossy(&fd).lines().filter(|l| !l.trim().is_empty()).count();
        print_str(&alloc::format!("files = {}\n", count));
    }
}

fn cmd_update() {
    ensure_dir(REPO_DIR);
    let c = alloc::format!("{}\0", REPO_DIR);
    let fd = match open(&c, 0) {
        Ok(f) => f,
        Err(_) => { print_str(&alloc::format!("spkg: repo dir {} not found\n", REPO_DIR)); return; }
    };
    let mut buf = [0u8; 4096];
    let mut count = 0usize;
    loop {
        let r = unsafe { libsarga::syscall::syscall3(217, fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (r as i64) <= 0 { break; }
        let entries = &buf[..r as usize];
        let mut i = 0;
        while i + 16 <= entries.len() {
            let reclen = u16::from_le_bytes([entries[i+8], entries[i+9]]) as usize;
            let namelen = entries[i+10] as usize;
            if reclen < 11 || i + reclen > entries.len() { break; }
            if namelen > 0 {
                if let Ok(name) = core::str::from_utf8(&entries[i+11..i+11+namelen]) {
                    if name.ends_with(".skp") {
                        count += 1;
                    }
                }
            }
            i += reclen;
        }
    }
    close(fd).ok();
    print_str(&alloc::format!("spkg: repo has {} packages\n", count));
}

fn cmd_upgrade() {
    if geteuid() != 0 {
        print_str("spkg: must be root to upgrade packages\n");
        return;
    }
    ensure_dir(REPO_DIR);
    let c = alloc::format!("{}\0", REPO_DIR);
    let fd = match open(&c, 0) {
        Ok(f) => f,
        Err(_) => { print_str("spkg: no repo configured\n"); return; }
    };
    let mut buf = [0u8; 4096];
    let mut upgraded = 0u32;
    loop {
        let r = unsafe { libsarga::syscall::syscall3(217, fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (r as i64) <= 0 { break; }
        let entries = &buf[..r as usize];
        let mut i = 0;
        while i + 16 <= entries.len() {
            let reclen = u16::from_le_bytes([entries[i+8], entries[i+9]]) as usize;
            let namelen = entries[i+10] as usize;
            if reclen < 11 || i + reclen > entries.len() { break; }
            if namelen > 0 {
                if let Ok(name) = core::str::from_utf8(&entries[i+11..i+11+namelen]) {
                    if name.ends_with(".skp") {
                        let pkg_name = name.trim_end_matches(".skp");
                        let installed_ver = get_installed_version(pkg_name);
                        if installed_ver.is_some() {
                            let repo_skp = alloc::format!("{}{}", REPO_DIR, name);
                            let repo_data = match read_whole_path(&repo_skp) {
                                Ok(d) => d,
                                Err(_) => continue,
                            };
                            if let Some(hdr) = read_skp_header(&repo_data) {
                                if let Ok(s) = core::str::from_utf8(&repo_data[hdr.manifest_off as usize..][..hdr.manifest_size as usize]) {
                                    let meta = parse_manifest(s);
                                    if let Some(repo_ver) = meta.get("version") {
                                        if repo_ver > installed_ver.as_ref().unwrap_or(&String::new()) {
                                            print_str(&alloc::format!("spkg: upgrading {} ({} -> {})\n", pkg_name, installed_ver.unwrap_or_default(), repo_ver));
                                            let mut visiting = Vec::new();
                                            install_skp_file(&repo_skp, &mut visiting);
                                            upgraded += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            i += reclen;
        }
    }
    close(fd).ok();
    if upgraded == 0 {
        print_str("spkg: all packages up to date\n");
    }
}

fn cmd_search(term: &str) {
    ensure_dir(REPO_DIR);
    let c = alloc::format!("{}\0", REPO_DIR);
    let fd = match open(&c, 0) {
        Ok(f) => f,
        Err(_) => { print_str("spkg: no repo configured\n"); return; }
    };
    let mut buf = [0u8; 4096];
    let mut found = 0u32;
    let lower_term = term.to_lowercase();
    loop {
        let r = unsafe { libsarga::syscall::syscall3(217, fd as u64, buf.as_mut_ptr() as u64, 4096) };
        if (r as i64) <= 0 { break; }
        let entries = &buf[..r as usize];
        let mut i = 0;
        while i + 16 <= entries.len() {
            let reclen = u16::from_le_bytes([entries[i+8], entries[i+9]]) as usize;
            let namelen = entries[i+10] as usize;
            if reclen < 11 || i + reclen > entries.len() { break; }
            if namelen > 0 {
                if let Ok(name) = core::str::from_utf8(&entries[i+11..i+11+namelen]) {
                    if name.ends_with(".skp") {
                        let lower_name = name.to_lowercase();
                        if lower_name.contains(&lower_term) || lower_name.contains(&lower_term) {
                            let pkg_name = name.trim_end_matches(".skp");
                            let ver = get_installed_version(pkg_name).unwrap_or_default();
                            let installed = if is_installed(pkg_name) { "[installed]" } else { "" };
                            print_str(&alloc::format!("  {} {} {}\n", pkg_name, ver, installed));
                            found += 1;
                        }
                    }
                }
            }
            i += reclen;
        }
    }
    close(fd).ok();
    if found == 0 {
        print_str(&alloc::format!("spkg: no packages matching '{}'\n", term));
    }
}

fn cmd_build(dir: &str) {
    let dirname = dir.trim_end_matches('/');
    let pkg_name = dirname.split('/').last().unwrap_or("package");

    // Read package.toml if it exists
    let manifest = {
        let manifest_path = alloc::format!("{}/package.toml", dir);
        match read_whole_path(&manifest_path) {
            Ok(d) => String::from_utf8_lossy(&d).to_string(),
            Err(_) => alloc::format!("[package]\nname = \"{}\"\nversion = \"0.1.0\"\ndescription = \"\"\n", pkg_name),
        }
    };

    // Collect files to package (everything except the manifest itself)
    let all_files = collect_files(dir);
    let files: Vec<(String, Vec<u8>)> = all_files.into_iter()
        .filter(|(p, _)| !p.ends_with("/package.toml"))
        .collect();

    let skp = build_skp(&files, &manifest);

    let out_name = alloc::format!("{}.skp", pkg_name);
    write_file(&out_name, &skp, 0o644).ok();
    print_str(&alloc::format!("spkg: built {} ({} bytes, {} files)\n", out_name, skp.len(), files.len()));
}

fn cmd_verify(pkg_file: &str) {
    let data = match read_whole_path(pkg_file) {
        Ok(d) => d,
        Err(e) => { print_str(&alloc::format!("spkg: failed to read {}: {}\n", pkg_file, e)); return; }
    };
    let hdr = match read_skp_header(&data) {
        Some(h) => h,
        None => { print_str("spkg: invalid .skp file\n"); return; }
    };
    // Verify checksum
    let mut ck_data = Vec::new();
    ck_data.extend_from_slice(&data[..48]);
    ck_data.extend_from_slice(&data[56..]);
    let expected = fnv1a(&ck_data);
    if expected == hdr.checksum {
        print_str("spkg: checksum OK\n");
    } else {
        print_str(&alloc::format!("spkg: checksum MISMATCH (expected {:#x}, got {:#x})\n", hdr.checksum, expected));
        return;
    }
    // Print manifest summary
    let manifest_str = match core::str::from_utf8(&data[hdr.manifest_off as usize..][..hdr.manifest_size as usize]) {
        Ok(s) => s,
        Err(_) => { print_str("spkg: invalid manifest encoding\n"); return; }
    };
    let meta = parse_manifest(manifest_str);
    let name = meta.get("name").map(|s| s.as_str()).unwrap_or("?");
    let ver = meta.get("version").map(|s| s.as_str()).unwrap_or("?");
    let psz = hdr.payload_size;
    print_str(&alloc::format!("spkg: package {} v{} ({} bytes payload)\n", name, ver, psz));
}

// ── entry point ─────────────────────────────────────────────────────────────

fn user_main() {
    let argc = args::argc();
    if argc < 2 {
        print_str("Usage: spkg <command> [args]\n");
        print_str("Commands:\n");
        print_str("  install <file.skp>  - Install a package (with deps)\n");
        print_str("  remove <name>       - Uninstall a package\n");
        print_str("  list                - List installed packages\n");
        print_str("  info <name>         - Show package details\n");
        print_str("  build <dir>         - Build .skp from directory\n");
        print_str("  verify <file.skp>   - Verify package integrity\n");
        print_str("  update              - Scan repo directory for packages\n");
        print_str("  upgrade             - Upgrade all installed packages from repo\n");
        print_str("  search <term>       - Search packages in repo\n");
        return;
    }

    // Ensure DB dir exists
    ensure_dir(DB_BASE);

    match args::get(1).unwrap_or("") {
        "install" => {
            if argc < 3 { print_str("Usage: spkg install <file.skp>\n"); return; }
            cmd_install(args::get(2).unwrap_or(""));
        }
        "remove" => {
            if argc < 3 { print_str("Usage: spkg remove <name>\n"); return; }
            cmd_remove(args::get(2).unwrap_or(""));
        }
        "list" | "ls" => cmd_list(),
        "info" => {
            if argc < 3 { print_str("Usage: spkg info <name>\n"); return; }
            cmd_info(args::get(2).unwrap_or(""));
        }
        "build" => {
            if argc < 3 { print_str("Usage: spkg build <dir>\n"); return; }
            cmd_build(args::get(2).unwrap_or("."));
        }
        "verify" => {
            if argc < 3 { print_str("Usage: spkg verify <file.skp>\n"); return; }
            cmd_verify(args::get(2).unwrap_or(""));
        }
        "update" => cmd_update(),
        "upgrade" => cmd_upgrade(),
        "search" => {
            if argc < 3 { print_str("Usage: spkg search <term>\n"); return; }
            cmd_search(args::get(2).unwrap_or(""));
        }
        _ => {
            print_str(&alloc::format!("spkg: unknown command: {}\n", args::get(1).unwrap_or("")));
        }
    }
}

sarga_main!(user_main);
