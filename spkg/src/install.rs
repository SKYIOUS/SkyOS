use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::io;
use libsarga::net::HttpClient;

use crate::db::{self, InstalledEntry};
use crate::repo::{RepoConfig, RepoIndexEntry};

pub fn download_package(
    repo: &RepoConfig,
    entry: &RepoIndexEntry,
) -> Result<Vec<u8>, &'static str> {
    let url = alloc::format!("{}{}", repo.url, entry.filename);
    libsarga::println!("spkg: downloading {} {}...", entry.name, entry.version);
    HttpClient::get(&url).map_err(|_| "download failed")
}

fn ensure_dir(path: &str) {
    let c = alloc::ffi::CString::new(path.as_bytes()).ok();
    if let Some(p) = c {
        let _ = unsafe { libsarga::syscall::syscall2(83, p.as_ptr() as u64, 0o755) };
    }
}

fn mkparent(path: &str) {
    if let Some(slash) = path.rfind('/') {
        let parent = &path[..slash];
        if parent.len() > 1 {
            ensure_dir(parent);
        }
    }
}

fn write_file_at(path: &str, data: &[u8]) -> Result<(), &'static str> {
    mkparent(path);
    let c = alloc::ffi::CString::new(path.as_bytes()).map_err(|_| "bad path")?;
    let fd = unsafe { libsarga::syscall::syscall2(2, c.as_ptr() as u64, 0x42) }; // O_RDWR | O_CREAT
    if fd < 0 {
        return Err("cannot create file");
    }
    let _ = unsafe {
        libsarga::syscall::syscall3(1, fd as u64, data.as_ptr() as u64, data.len() as u64)
    };
    let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
    Ok(())
}

pub fn extract_tar(data: &[u8]) -> Result<Vec<String>, &'static str> {
    const BLOCK: usize = 512;
    let mut off = 0;
    let mut files = Vec::new();
    while off + BLOCK <= data.len() {
        let hdr = &data[off..off + BLOCK];
        if hdr.iter().all(|&b| b == 0) {
            break;
        }
        let magic = core::str::from_utf8(&hdr[257..262]).unwrap_or("");
        if magic != "ustar" {
            return Err("invalid tar archive");
        }
        let name_end = hdr.iter().position(|&b| b == 0).unwrap_or(100);
        let name = core::str::from_utf8(&hdr[..name_end]).unwrap_or("");
        let size = crate::parse_octal(&hdr[124..136]) as usize;
        let typeflag = hdr[156];
        let file_data = &data[off + BLOCK..off + BLOCK + size];
        if file_data.len() < size {
            break;
        }
        match typeflag {
            b'5' => {
                ensure_dir(name);
            }
            b'0' | b'\0' => {
                write_file_at(name, file_data)?;
                files.push(name.to_string());
            }
            _ => {}
        }
        let advance = BLOCK + size.div_ceil(BLOCK) * BLOCK;
        off += advance;
    }
    Ok(files)
}

pub fn split_spkg(data: &[u8]) -> (&[u8], Option<&[u8]>) {
    for i in 0..data.len().saturating_sub(4) {
        if data[i] == b'-' && data[i + 1] == b'-' && data[i + 2] == b'-' && data[i + 3] == b'\n' {
            return (&data[..i], Some(&data[i + 4..]));
        }
    }
    (data, None) // no manifest, entire file is tar
}

pub fn install_package(
    data: &[u8],
    entry: &RepoIndexEntry,
    db: &mut Vec<InstalledEntry>,
) -> Result<(), &'static str> {
    libsarga::println!("spkg: installing {} v{}...", entry.name, entry.version);
    let (_manifest, tar_data) = split_spkg(data);
    let tar = tar_data.unwrap_or(data);
    let files = extract_tar(tar)?;
    let deps = entry
        .dependencies
        .iter()
        .map(|d| {
            let space = d.find(' ').unwrap_or(d.len());
            d[..space].to_string()
        })
        .collect();
    db.push(InstalledEntry {
        name: entry.name.clone(),
        version: entry.version.clone(),
        files,
        dependencies: deps,
    });
    db::save_db(db).ok();
    libsarga::println!("spkg: {} v{} installed", entry.name, entry.version);
    Ok(())
}

pub fn remove_package(
    entry: &InstalledEntry,
    db: &mut Vec<InstalledEntry>,
) -> Result<(), &'static str> {
    libsarga::println!("spkg: removing {} v{}...", entry.name, entry.version);
    for f in &entry.files {
        let c = alloc::ffi::CString::new(f.as_bytes()).map_err(|_| "bad path")?;
        let _ = unsafe { libsarga::syscall::syscall1(87, c.as_ptr() as u64) }; // SYS_UNLINK
    }
    db.retain(|e| e.name != entry.name);
    db::save_db(db).ok();
    libsarga::println!("spkg: {} removed", entry.name);
    Ok(())
}

pub fn fetch_cached_spkg(
    _repo_name: &str,
    entry: &RepoIndexEntry,
) -> Result<Vec<u8>, &'static str> {
    let cache_path = alloc::format!("/var/spkg/cache/{}", entry.filename);
    let fd = io::open(&cache_path, 0).map_err(|_| "package not in cache")?;
    let mut data = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match io::read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    io::close(fd).ok();
    Ok(data)
}

pub fn cache_spkg_data(_repo: &RepoConfig, entry: &RepoIndexEntry, data: &[u8]) {
    let cache_path = alloc::format!("/var/spkg/cache/{}", entry.filename);
    let _ = io::mkdir("/var/spkg/cache/", 0o755);
    let fd = io::open(&cache_path, 0x42).ok();
    if let Some(f) = fd {
        let _ = io::write(f, data);
        io::close(f).ok();
    }
}
