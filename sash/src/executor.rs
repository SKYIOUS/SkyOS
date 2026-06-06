#![no_std]
use libsky::{process, println};
use alloc::string::String;
use alloc::vec::Vec;

pub fn execute(cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() { return; }
    
    // Builtin check
    if crate::builtins::handle_builtin(&parts) {
        return;
    }

    if let Ok(pid) = process::fork() {
        if pid == 0 {
            // child
            let _ = process::execve(parts[0], &parts, &[]); // ignore envp for now
            println!("ash: command not found: {}", parts[0]);
            process::exit(127);
        } else {
            // parent
            let _ = process::wait(pid);
        }
    } else {
        println!("ash: fork failed");
    }
}
