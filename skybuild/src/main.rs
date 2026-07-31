#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::io;
use libsarga::process;
use libsarga::sarga_main;

fn puts(s: &str) {
    io::print_str(s);
    io::print_str("\n");
}

fn read_file(path: &str) -> String {
    io::read_to_string(path).unwrap_or_default()
}

fn run_cmd(cmd: &str, args: &[&str]) -> i64 {
    match process::fork() {
        Ok(0) => {
            let _ = process::execve(cmd, args, &[]);
            0
        }
        Ok(pid) => process::wait(pid).unwrap_or(-1) as i64,
        Err(_) => -1,
    }
}

fn cmd_build(args: &[&str]) {
    if args.is_empty() {
        puts("Usage: skybuild build <recipe>");
        return;
    }
    let recipe_name = args[0];
    let recipe_path = if recipe_name.contains('/') || recipe_name.ends_with(".recipe") {
        recipe_name.to_string()
    } else {
        alloc::format!("{}.recipe", recipe_name)
    };

    let recipe = read_file(&recipe_path);
    if recipe.is_empty() {
        puts(&alloc::format!("Cannot read recipe: {}", recipe_path));
        return;
    }

    // Parse name from recipe
    let mut pkg_name = recipe_name;
    for line in recipe.lines() {
        if let Some(name_val) = line.strip_prefix("name=\"") {
            if let Some(end) = name_val.find('"') {
                pkg_name = &name_val[..end];
                break;
            }
        }
    }

    puts(&alloc::format!("Building package: {}", pkg_name));

    // Build with Cargo target
    let status = run_cmd("/bin/cargo", &[
        "build", "--target", "x86_64-skyos", "--release",
        "-Z", "build-std=core,alloc",
    ]);
    if status != 0 {
        puts(&alloc::format!("Build failed (status {})", status));
        return;
    }

    // Package binary into .skp format
    let binary_path = alloc::format!("target/x86_64-skyos/release/{}", pkg_name);
    let skp_path = alloc::format!("{}.skp", pkg_name);
    let pkg_status = run_cmd("/bin/spkg", &["pack", &binary_path, "-o", &skp_path]);
    if pkg_status == 0 {
        puts(&alloc::format!("Created package: {}", skp_path));
    } else {
        puts("Package creation failed");
    }
}

fn cmd_new(args: &[&str]) {
    let name = if args.is_empty() { "myapp" } else { args[0] };
    let recipe = alloc::format!(
        "name=\"{}\"\nversion=\"1.0.0\"\ndescription=\"A SkyOS application\"\narch=\"x86_64\"\nlicense=\"MIT\"\ndeps=\"\"\nmaintainer=\"developer\"\nsize=0\nsha256=\"\"\npayload:\n",
        name
    );
    match io::open(name, 0x42) {
        Ok(fd) => {
            let _ = io::write_all(fd, recipe.as_bytes());
            let _ = io::close(fd);
            puts(&alloc::format!("Created recipe: {}", name));
        }
        Err(_) => puts("Cannot create recipe"),
    }
}

fn cmd_init(_args: &[&str]) {
    puts("SkyOS Developer Toolchain (skybuild)");
    puts("");
    puts("Available:");
    puts("  skybuild new <name>       Create a new recipe scaffold");
    puts("  skybuild build <name>     Build and package a recipe");
    puts("  skybuild install <name>   Install a built package");
    puts("  skybuild repo <path>      Generate local repo from packages");
    puts("  skybuild sysroot          Show SDK path");
    puts("  skybuild info             Show toolchain info");
}

fn cmd_sysroot(_args: &[&str]) {
    puts("/usr");
    puts("");
    puts("SDK structure:");
    puts("  /usr/include/   C/C++ headers");
    puts("  /usr/lib/       Static libraries");
    puts("  /usr/share/     Shared data");
    puts("  /bin/           Build tools");
}

fn cmd_info(_args: &[&str]) {
    puts("SkyOS Developer Toolchain");
    puts("Target: x86_64-skyos");
    puts("C Library: libsarga (Rust, no_std)");
    puts("Package format: .skp (spkg)");
    puts("Build system: Cargo + custom target JSON");
}

fn cmd_install(args: &[&str]) {
    if args.is_empty() {
        puts("Usage: skybuild install <package>");
        return;
    }
    let pkg_name = args[0];
    let skp_path = if pkg_name.ends_with(".skp") {
        pkg_name.to_string()
    } else {
        alloc::format!("{}.skp", pkg_name)
    };
    let status = run_cmd("/bin/spkg", &["install", &skp_path]);
    if status == 0 {
        puts(&alloc::format!("Installed: {}", pkg_name));
    } else {
        puts(&alloc::format!("Install failed (status {})", status));
    }
}

fn cmd_repo(args: &[&str]) {
    let repo_path = args.first().copied().unwrap_or("/repo");
    puts(&alloc::format!("Generating local repo at: {}", repo_path));
    let status = run_cmd("/bin/spkg", &["repo-index", repo_path]);
    if status == 0 {
        puts("Repository index created.");
    } else {
        puts("Repository index failed");
    }
}

fn user_main() -> i32 {
    let mut args = Vec::new();
    for i in 1..libsarga::args::argc() {
        args.push(
            libsarga::args::get(i as usize)
                .unwrap_or_default()
                .to_string(),
        );
    }
    let args_str: Vec<&str> = args.iter().map(|s: &String| s.as_str()).collect();
    let cmd = args_str.first().copied().unwrap_or("");
    match cmd {
        "build" => cmd_build(&args_str[1..]),
        "install" => cmd_install(&args_str[1..]),
        "repo" => cmd_repo(&args_str[1..]),
        "new" => cmd_new(&args_str[1..]),
        "sysroot" => cmd_sysroot(&args_str[1..]),
        "info" => cmd_info(&args_str[1..]),
        _ => cmd_init(&args_str[1..]),
    }
    0
}

sarga_main!(user_main);
