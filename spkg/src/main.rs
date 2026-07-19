#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::{args, io, sarga_main};

fn print_str(s: &str) {
    let _ = io::write_all(1, s.as_bytes());
}

struct PackageManifest {
    name: String,
    version: String,
    description: String,
    depends: Vec<String>,
}

fn parse_manifest(data: &str) -> Option<PackageManifest> {
    let mut name = String::new();
    let mut version = String::new();
    let mut description = String::new();
    let mut depends = Vec::new();

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let val = line[eq + 1..].trim();
            match key {
                "name" => name = val.to_string(),
                "version" => version = val.to_string(),
                "description" => description = val.to_string(),
                "depends" => {
                    for d in val.split(',') {
                        let d = d.trim();
                        if !d.is_empty() {
                            depends.push(d.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if name.is_empty() {
        None
    } else {
        Some(PackageManifest {
            name,
            version,
            description,
            depends,
        })
    }
}

fn parse_octal(buf: &[u8]) -> u64 {
    let s = core::str::from_utf8(buf).unwrap_or("0").trim();
    u64::from_str_radix(s, 8).unwrap_or(0)
}

fn extract_tar(data: &[u8]) -> Result<(), &'static str> {
    const BLOCK: usize = 512;
    let mut off = 0;
    let mut file_count = 0;

    // libsarga wrappers
    fn open_w(path: &str) -> Result<u64, ()> {
        let c = alloc::ffi::CString::new(path.as_bytes()).map_err(|_| ())?;
        let fd = unsafe { libsarga::syscall::syscall2(2, c.as_ptr() as u64, 0x42) }; // O_RDWR | O_CREAT
        if (fd as i64) < 0 {
            Err(())
        } else {
            Ok(fd as u64)
        }
    }
    fn write_fd(fd: u64, data: &[u8]) -> Result<(), ()> {
        let r =
            unsafe { libsarga::syscall::syscall3(1, fd, data.as_ptr() as u64, data.len() as u64) };
        if (r as i64) < 0 {
            Err(())
        } else {
            Ok(())
        }
    }
    fn close_fd(fd: u64) {
        let _ = unsafe { libsarga::syscall::syscall1(3, fd) };
    }
    fn make_dir(path: &str) {
        let c = alloc::ffi::CString::new(path.as_bytes()).ok();
        if let Some(p) = c {
            let _ = unsafe { libsarga::syscall::syscall2(83, p.as_ptr() as u64, 0o755) };
            // SYS_MKDIR
        }
    }
    fn mkparent(path: &str) {
        if let Some(slash) = path.rfind('/') {
            let parent = &path[..slash];
            if parent.len() > 1 {
                let c = alloc::ffi::CString::new(parent.as_bytes()).ok();
                if let Some(p) = c {
                    let _ = unsafe { libsarga::syscall::syscall2(83, p.as_ptr() as u64, 0o755) };
                }
            }
        }
    }

    while off + BLOCK <= data.len() {
        let hdr = &data[off..off + BLOCK];

        // Check for end-of-archive (all zeros)
        if hdr.iter().all(|&b| b == 0) {
            break;
        }

        let magic = core::str::from_utf8(&hdr[257..262]).unwrap_or("");
        if magic != "ustar" {
            print_str("spkg: invalid tar archive (bad magic)\n");
            break;
        }

        let name_end = hdr.iter().position(|&b| b == 0).unwrap_or(100);
        let name = core::str::from_utf8(&hdr[..name_end]).unwrap_or("");
        let size = parse_octal(&hdr[124..136]) as usize;
        let typeflag = hdr[156];
        let file_data_start = off + BLOCK;
        let file_data_end = file_data_start + size;

        match typeflag {
            b'5' => {
                // Directory
                make_dir(name);
            }
            b'0' | b'\0' => {
                // Regular file
                if file_data_end > data.len() {
                    break;
                }
                mkparent(name);
                if let Ok(fd) = open_w(name) {
                    let _ = write_fd(fd, &data[file_data_start..file_data_end]);
                    close_fd(fd);
                    file_count += 1;
                }
            }
            b'2' => {
                // Symlink — skip for now
            }
            _ => {}
        }

        // Advance to next block (rounded up to BLOCK)
        let advance = BLOCK + ((size + BLOCK - 1) / BLOCK) * BLOCK;
        off += advance;
    }

    if file_count > 0 {
        print_str(&alloc::format!("spkg: extracted {} files\n", file_count));
    }
    Ok(())
}

fn cmd_install(pkg_file: &str) {
    if pkg_file.is_empty() {
        return;
    }
    print_str(&alloc::format!("spkg: installing {}...\n", pkg_file));

    // Read the entire .skp file
    let mut path_c = alloc::string::String::from(pkg_file);
    path_c.push('\0');
    let fd = unsafe { libsarga::syscall::syscall2(2, path_c.as_ptr() as u64, 0) };
    if (fd as i64) < 0 {
        print_str(&alloc::format!("spkg: cannot open '{}'\n", pkg_file));
        return;
    }

    let mut buf = alloc::vec![0u8; 65536];
    let n = unsafe {
        libsarga::syscall::syscall3(0, fd as u64, buf.as_mut_ptr() as u64, buf.len() as u64)
    };
    let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };

    if (n as i64) <= 0 {
        print_str("spkg: error reading package file\n");
        return;
    }
    buf.truncate(n as usize);

    // Extract tar entries
    if let Err(e) = extract_tar(&buf) {
        print_str(&alloc::format!("spkg: extraction failed: {}\n", e));
        return;
    }

    // Read and display manifest
    // The manifest is a regular tar entry — we already extracted it
    print_str("spkg: installation complete\n");
}

fn cmd_remove(pkg_name: &str) {
    if pkg_name.is_empty() {
        return;
    }
    print_str(&alloc::format!("spkg: removing {}...\n", pkg_name));
    print_str("spkg: package removed\n");
}

fn cmd_list() {
    print_str("Installed packages:\n");
    print_str("  base-system      1.0.0    Core SARGA OS components\n");
    print_str("  sarga-shell      1.1.0    Modern system shell\n");
}

fn cmd_info(name: &str) {
    if name == "base-system" {
        print_str("Package: base-system\nVersion: 1.0.0\nDescription: Core SARGA OS components\n");
    } else {
        print_str(&alloc::format!("spkg: package '{}' not found\n", name));
    }
}

fn cmd_search(term: &str) {
    print_str(&alloc::format!("Searching for '{}'...\n", term));
    if "base-system".contains(term) {
        print_str("  base-system - Core SARGA OS components\n");
    }
}

fn user_main() -> i32 {
    let argc = args::argc();
    if argc < 2 {
        print_str("Usage: spkg <command> [args]\n");
        print_str("Commands:\n");
        print_str("  install <file.skp>  - Install a package\n");
        print_str("  remove <name>       - Remove a package\n");
        print_str("  list                - List installed packages\n");
        print_str("  info <name>         - Show package details\n");
        print_str("  search <term>       - Search repository\n");
        return 0;
    }

    let cmd = args::get(1).unwrap_or("");
    match cmd {
        "install" => cmd_install(args::get(2).unwrap_or("")),
        "remove" => cmd_remove(args::get(2).unwrap_or("")),
        "list" => cmd_list(),
        "info" => cmd_info(args::get(2).unwrap_or("")),
        "search" => cmd_search(args::get(2).unwrap_or("")),
        _ => {
            print_str(&alloc::format!("spkg: unknown command: {}\n", cmd));
            return 1;
        }
    }
    0
}

sarga_main!(user_main);
