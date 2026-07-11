#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::sarga_main;
use libsarga::io;
use libsarga::process;

const PKG_DB_DIR: &str = "/var/cache/skypkg";
const PKG_INSTALLED_DIR: &str = "/var/cache/skypkg/installed";
const REPO_CATALOG: &str = "/var/cache/skypkg/repo.catalog";
const DEFAULT_REPO_URL: &str = "https://packages.skyos.dev/catalog.json";
const SKP_MAGIC: [u8; 4] = *b"SKP1";

struct Manifest {
    name: String,
    version: String,
    description: String,
    deps: Vec<String>,
    arch: String,
    size: u64,
    sha256: String,
    maintainer: String,
    license: String,
}

fn user_main() -> i32 {
    let mut args = Vec::new();
    for i in 1..libsarga::args::argc() {
        args.push(libsarga::args::get(i as usize).unwrap_or_default().to_string());
    }
    if args.is_empty() {
        io::print_str("Usage: skypkg <command> [args]\n");
        return 1;
    }
    0
}

sarga_main!(user_main);
