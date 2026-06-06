#![no_std]
#![no_main]

use libsarga::sarga_main;
use libsarga::io;
extern crate alloc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;

sarga_main!(user_main);

fn user_main() {
    io::write_all(1, b"Sarga Shell (sash) v0.1.0\n").ok();
    
    let mut input_buffer = String::new();
    
    loop {
        io::write_all(1, b"sash> ").ok();
        
        let mut buf = [0u8; 1024];
        match io::read(0, &mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => {
                let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
                for c in s.chars() {
                    if c == '\n' || c == '\r' {
                        if !input_buffer.is_empty() {
                            execute_command(&input_buffer);
                            input_buffer.clear();
                        }
                        io::write_all(1, b"sash> ").ok();
                    } else {
                        input_buffer.push(c);
                    }
                }
            }
            Err(_) => break,
        }
    }
}

fn execute_command(cmd_line: &str) {
    let parts: Vec<&str> = cmd_line.split_whitespace().collect();
    if parts.is_empty() { return; }
    
    let cmd = parts[0];
    let args = &parts[1..];
    
    match cmd {
        "exit" => libsarga::process::exit(0),
        "help" => {
            io::write_all(1, b"Sarga Shell (sash) Commands:\n").ok();
            io::write_all(1, b"  help     - Show this help\n").ok();
            io::write_all(1, b"  exit     - Exit the shell\n").ok();
            io::write_all(1, b"  ai       - Query VahiAI\n").ok();
            io::write_all(1, b"  [cmd]    - Execute system command\n").ok();
        }
        "ai" => {
            if args.is_empty() {
                io::write_all(1, b"Usage: ai <intent> [args...]\n").ok();
                return;
            }
            let intent = args[0];
            match libsarga::ai::query(intent) {
                Ok(resp) => {
                    io::write_all(1, b"VahiAI: ").ok();
                    io::write_all(1, resp.as_bytes()).ok();
                    io::write_all(1, b"\n").ok();
                }
                Err(_e) => {
                    io::write_all(1, b"VahiAI: Error\n").ok();
                }
            }
        }
        _ => {
            // Try to execute as an external command
            match libsarga::process::fork() {
                Ok(0) => {
                    let mut env = Vec::new();
                    env.push("PATH=/bin:/usr/bin");
                    libsarga::process::execve(cmd, parts.as_slice(), env.as_slice());
                    io::write_all(1, b"sash: command not found: ").ok();
                    io::write_all(1, cmd.as_bytes()).ok();
                    io::write_all(1, b"\n").ok();
                    libsarga::process::exit(1);
                }
                Ok(pid) => {
                    let _ = libsarga::process::wait(pid);
                }
                Err(_) => {
                    io::write_all(1, b"sash: fork failed\n").ok();
                }
            }
        }
    }
}
