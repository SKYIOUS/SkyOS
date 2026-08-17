#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::{args, io, sarga_main};

mod db;
mod deps;
mod install;
mod repo;

fn print_usage() {
    io::print_str("Usage: spkg <command> [args]\nCommands:\n");
    io::print_str("  install <pkg>        - Install a package (from repo or .spkg file)\n");
    io::print_str("  remove <pkg>         - Remove a package\n");
    io::print_str("  list                 - List installed packages\n");
    io::print_str("  info <pkg>           - Show package details\n");
    io::print_str("  search <term>        - Search repository\n");
    io::print_str("  update               - Fetch repository index from all enabled repos\n");
    io::print_str("  upgrade              - Upgrade all installed packages\n");
}

fn cmd_update() {
    let repos = repo::load_repos();
    if repos.is_empty() {
        io::print_str("spkg: no repositories configured in /etc/spkg/repos.conf\n");
        return;
    }
    for r in &repos {
        if !r.enabled {
            continue;
        }
        libsarga::println!("spkg: fetching index from {} ({})...", r.name, r.url);
        match repo::fetch_and_cache_index(r) {
            Ok(entries) => {
                libsarga::println!("spkg: {} packages available from {}", entries.len(), r.name)
            }
            Err(e) => libsarga::println!("spkg: {}: {}", r.name, e),
        }
    }
    io::print_str("spkg: update complete\n");
}

fn cmd_search(term: &str) {
    let repos = repo::load_repos();
    let mut found = false;
    for r in &repos {
        if !r.enabled {
            continue;
        }
        let entries = match repo::load_cached_index(&r.name) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for e in &entries {
            if e.name.contains(term) || e.description.contains(term) {
                libsarga::println!(
                    "  {} {} - {} ({})",
                    e.name,
                    e.version,
                    e.description,
                    r.name
                );
                found = true;
            }
        }
    }
    if !found {
        io::print_str("spkg: no packages found\n");
    }
}

fn cmd_info(name: &str) {
    // Check installed first
    let db_entries = db::load_db();
    if let Some(inst) = db::get_installed(&db_entries, name) {
        libsarga::println!("Package: {} (installed)", inst.name);
        libsarga::println!("Version: {}", inst.version);
        libsarga::println!("Files: {}", inst.files.len());
        io::print_str("Dependencies: ");
        for d in &inst.dependencies {
            libsarga::print!("{} ", d);
        }
        io::print_str("\n");
        return;
    }
    // Search in repos
    let repos = repo::load_repos();
    for r in &repos {
        if !r.enabled {
            continue;
        }
        let entries = match repo::load_cached_index(&r.name) {
            Ok(e) => e,
            Err(_) => continue,
        };
        if let Some(e) = repo::find_in_index(&entries, name) {
            libsarga::println!("Package: {} ({})", e.name, r.name);
            libsarga::println!("Version: {}", e.version);
            libsarga::println!("Description: {}", e.description);
            io::print_str("Dependencies: ");
            for d in &e.dependencies {
                libsarga::print!("{} ", d);
            }
            io::print_str("\n");
            return;
        }
    }
    libsarga::println!("spkg: package '{}' not found", name);
}

fn cmd_list() {
    let db_entries = db::load_db();
    if db_entries.is_empty() {
        io::print_str("No packages installed\n");
        return;
    }
    io::print_str("Installed packages:\n");
    for e in &db_entries {
        libsarga::println!("  {} {} - {} files", e.name, e.version, e.files.len());
    }
}

fn get_all_index_entries() -> Vec<(String, repo::RepoIndexEntry)> {
    let mut all = Vec::new();
    let repos = repo::load_repos();
    for r in &repos {
        if !r.enabled {
            continue;
        }
        let entries = match repo::load_cached_index(&r.name) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for e in entries {
            all.push((r.name.clone(), e));
        }
    }
    all
}

fn find_entry_in_all(name: &str) -> Option<(String, repo::RepoIndexEntry)> {
    get_all_index_entries()
        .into_iter()
        .find(|(_, e)| e.name == name)
}

fn cmd_install(name: &str) {
    if name.is_empty() {
        io::print_str("spkg: specify a package name or .spkg file\n");
        return;
    }
    // If it's a .spkg file, install from local file
    if name.ends_with(".spkg") || name.ends_with(".skp") {
        install_local_file(name);
        return;
    }
    // Install from repo
    let mut db_entries = db::load_db();
    let (repo_name, entry) = match find_entry_in_all(name) {
        Some(e) => e,
        None => {
            libsarga::println!("spkg: package '{}' not found in any repository", name);
            return;
        }
    };
    // Resolve dependencies
    let repos = repo::load_repos();
    let all_entries = get_all_index_entries();
    let index: Vec<repo::RepoIndexEntry> = all_entries.into_iter().map(|(_, e)| e).collect();
    let to_install =
        match deps::resolve_all(core::slice::from_ref(&entry.name), &index, &db_entries) {
            Ok(order) => order,
            Err(e) => {
                libsarga::println!("spkg: dependency error: {}", e);
                return;
            }
        };
    for pkg_name in &to_install {
        if db::is_installed(&db_entries, pkg_name) {
            continue;
        }
        let entry = match repo::find_in_index(&index, pkg_name) {
            Some(e) => e,
            None => {
                libsarga::println!("spkg: package '{}' not found", pkg_name);
                continue;
            }
        };
        // Find which repo has it
        let repo = match repos.iter().find(|r2| r2.name == repo_name) {
            Some(r) => r,
            None => {
                libsarga::println!("spkg: repo '{}' not found", repo_name);
                continue;
            }
        };
        // Try cache first, then download
        let data = match install::fetch_cached_spkg(&repo.name, entry) {
            Ok(d) => d,
            Err(_) => match install::download_package(repo, entry) {
                Ok(d) => {
                    install::cache_spkg_data(repo, entry, &d);
                    d
                }
                Err(e) => {
                    libsarga::println!("spkg: download failed: {}", e);
                    continue;
                }
            },
        };
        if let Err(e) = install::install_package(&data, entry, &mut db_entries) {
            libsarga::println!("spkg: install failed: {}", e);
        }
    }
}

fn install_local_file(path: &str) {
    let fd = match io::open(path, 0) {
        Ok(f) => f,
        Err(_) => {
            libsarga::println!("spkg: cannot open '{}'", path);
            return;
        }
    };
    let mut data = Vec::new();
    let mut buf = [0u8; 65536];
    loop {
        match io::read(fd, &mut buf) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    io::close(fd).ok();
    let mut db_entries = db::load_db();
    let (_manifest, tar_data) = install::split_spkg(&data);
    let tar = tar_data.unwrap_or(&data);
    let manifest_str = if !_manifest.is_empty() {
        core::str::from_utf8(_manifest).ok()
    } else {
        None
    };
    let pkg_name = manifest_str
        .and_then(|s| parse_manifest_val(s, "name"))
        .map(|s| s.to_string())
        .unwrap_or_else(|| guess_name(path));
    let pkg_version = manifest_str
        .and_then(|s| parse_manifest_val(s, "version"))
        .unwrap_or("0.0.0");
    let files = match install_extract_tar(tar) {
        Ok(f) => f,
        Err(e) => {
            libsarga::println!("spkg: extraction failed: {}", e);
            return;
        }
    };
    db_entries.push(db::InstalledEntry {
        name: pkg_name.clone(),
        version: pkg_version.to_string(),
        files,
        dependencies: Vec::new(),
    });
    db::save_db(&db_entries).ok();
    libsarga::println!(
        "spkg: {} v{} installed from local file",
        pkg_name,
        pkg_version
    );
}

fn parse_manifest_val<'a>(data: &'a str, key: &str) -> Option<&'a str> {
    for line in data.lines() {
        let line = line.trim();
        if let Some(eq) = line.find('=') {
            if line[..eq].trim() == key {
                return Some(line[eq + 1..].trim().trim_matches('"'));
            }
        }
    }
    None
}

fn guess_name(path: &str) -> String {
    let base = path.rsplit('/').next().unwrap_or(path);
    let dot = base.find('.').unwrap_or(base.len());
    base[..dot].to_string()
}

fn install_extract_tar(data: &[u8]) -> Result<alloc::vec::Vec<String>, &'static str> {
    install::extract_tar(data)
}

fn cmd_remove(name: &str) {
    let mut db_entries = db::load_db();
    let idx = db_entries.iter().position(|e| e.name == name);
    let idx = match idx {
        Some(i) => i,
        None => {
            libsarga::println!("spkg: package '{}' not installed", name);
            return;
        }
    };
    let entry = db_entries[idx].clone();
    let _ = install::remove_package(&entry, &mut db_entries);
}

fn cmd_upgrade() {
    let db_entries = db::load_db();
    if db_entries.is_empty() {
        io::print_str("spkg: no packages installed\n");
        return;
    }
    let repos = repo::load_repos();
    let all_entries = get_all_index_entries();
    let mut upgraded = 0;
    for inst in &db_entries {
        for (repo_name, entry) in &all_entries {
            if entry.name != inst.name {
                continue;
            }
            let iv = libsarga::semver::Version::parse(&inst.version);
            let ev = libsarga::semver::Version::parse(&entry.version);
            if let (Some(iv), Some(ev)) = (iv, ev) {
                if ev.compare(&iv) > 0 {
                    libsarga::println!(
                        "spkg: upgrading {} {} -> {}",
                        inst.name,
                        inst.version,
                        entry.version
                    );
                    let repo = match repos.iter().find(|r| r.name == *repo_name) {
                        Some(r) => r,
                        None => {
                            libsarga::println!("spkg: repo '{}' not found", repo_name);
                            continue;
                        }
                    };
                    let data = match install::download_package(repo, entry) {
                        Ok(d) => d,
                        Err(e) => {
                            libsarga::println!("spkg: download failed: {}", e);
                            continue;
                        }
                    };
                    let mut db = db::load_db();
                    let _old = db::get_installed(&db, &entry.name).map(|e| {
                        for f in &e.files {
                            let bytes: &[u8] = f.as_bytes();
                            if let Ok(c) = alloc::ffi::CString::new(bytes) {
                                let _ =
                                    unsafe { libsarga::syscall::syscall1(87, c.as_ptr() as u64) };
                            }
                        }
                    });
                    db.retain(|e| e.name != entry.name);
                    if let Err(e) = install::install_package(&data, entry, &mut db) {
                        libsarga::println!("spkg: upgrade failed: {}", e);
                    } else {
                        upgraded += 1;
                    }
                }
            }
        }
    }
    libsarga::println!("spkg: upgraded {} packages", upgraded);
}

fn parse_octal(buf: &[u8]) -> u64 {
    let s = core::str::from_utf8(buf).unwrap_or("0").trim();
    u64::from_str_radix(s, 8).unwrap_or(0)
}

fn user_main() -> i32 {
    let argc = args::argc();
    if argc < 2 {
        print_usage();
        return 0;
    }
    let cmd = args::get(1).unwrap_or("");
    match cmd {
        "update" => cmd_update(),
        "search" => cmd_search(args::get(2).unwrap_or("")),
        "info" => cmd_info(args::get(2).unwrap_or("")),
        "install" => cmd_install(args::get(2).unwrap_or("")),
        "remove" => cmd_remove(args::get(2).unwrap_or("")),
        "list" => cmd_list(),
        "upgrade" => cmd_upgrade(),
        _ => {
            libsarga::println!("spkg: unknown command: {}", cmd);
            return 1;
        }
    }
    0
}

sarga_main!(user_main);
