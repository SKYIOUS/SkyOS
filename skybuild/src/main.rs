#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libsarga::sarga_main;
use libsarga::io;
use libsarga::process;

fn puts(s: &str) { io::print_str(s); io::print_str("\n"); }

fn read_file(path: &str) -> String {
    io::read_to_string(path).unwrap_or_default()
}

fn run_cmd(cmd: &str, args: &[&str]) -> i64 {
    match process::fork() {
        Ok(0) => {
            let _ = process::execve(cmd, args, &[]);
            0
        }
        Ok(pid) => {
            process::wait(pid).unwrap_or(-1) as i64
        }
        Err(_) => -1,
    }
}

fn cmd_build(args: &[&str]) {
    if args.is_empty() { puts("Usage: skybuild <recipe>"); return; }
    let recipe = read_file(args[0]);
    if recipe.is_empty() { puts("Cannot read recipe"); return; }
    puts(&alloc::format!("Building from: {}", args[0]));
    puts("Running skypkg build...");
    let status = run_cmd("/bin/skypkg", &["build", args[0]]);
    if status == 0 {
        puts("Build succeeded.");
    } else {
        puts(&alloc::format!("Build failed (status {})", status));
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
    puts("  skybuild new <name>    Create a new recipe scaffold");
    puts("  skybuild build <file>  Build a recipe into .skp");
    puts("  skybuild sysroot       Show SDK path");
    puts("  skybuild info          Show toolchain info");
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
    puts("Package format: .skp (skypkg)");
    puts("Build system: Cargo + custom target JSON");
}

fn user_main() -> i32 {
    let mut args = Vec::new();
    for i in 1..libsarga::args::argc() {
        args.push(libsarga::args::get(i as usize).unwrap_or_default().to_string());
    }
    let args_str: Vec<&str> = args.iter().map(|s: &String| s.as_str()).collect();
    let cmd = args_str.first().copied().unwrap_or("");
    match cmd {
        "build" => cmd_build(&args_str[1..]),
        "new" => cmd_new(&args_str[1..]),
        "sysroot" => cmd_sysroot(&args_str[1..]),
        "info" => cmd_info(&args_str[1..]),
        _ => cmd_init(&args_str[1..]),
    }
    0
}

sarga_main!(user_main);
