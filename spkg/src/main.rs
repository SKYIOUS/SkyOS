#![no_std]
#![no_main]
extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::{sarga_main, io, args};

const REPO_DIR: &str = "/usr/pkg/repo";
const INSTALLED_DIR: &str = "/usr/pkg/installed";

fn print_str(s: &str) { let _ = io::write_all(1, s.as_bytes()); }

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
        if line.is_empty() || line.starts_with('#') { continue; }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let val = line[eq+1..].trim();
            match key {
                "name" => name = val.to_string(),
                "version" => version = val.to_string(),
                "description" => description = val.to_string(),
                "depends" => {
                    for d in val.split(',') {
                        let d = d.trim();
                        if !d.is_empty() { depends.push(d.to_string()); }
                    }
                }
                _ => {}
            }
        }
    }
    if name.is_empty() { None } else { Some(PackageManifest { name, version, description, depends }) }
}

fn cmd_install(pkg_file: &str) {
    if pkg_file.is_empty() { return 0; }
    print_str(&alloc::format!("spkg: installing {}...\n", pkg_file));
    // ... installation logic ...
    print_str("spkg: installation complete\n");
}

fn cmd_remove(pkg_name: &str) {
    if pkg_name.is_empty() { return 0; }
    print_str(&alloc::format!("spkg: removing {}...\n", pkg_name));
    // ... removal logic ...
}

fn cmd_list() {
    print_str("Installed packages:\n");
    // ... list logic ...
}

fn cmd_info(name: &str) {
    print_str(&alloc::format!("Package info for {}:\n", name));
    // ... info logic ...
}

fn cmd_search(term: &str) {
    print_str(&alloc::format!("Searching for '{}'...\n", term));
    // ... search logic ...
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
