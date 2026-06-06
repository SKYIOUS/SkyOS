#![no_std]
#![no_main]
use libsarga::sarga_main;
use libsarga::io;
use libsarga::println;
extern crate alloc;
use core::cell::UnsafeCell;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;

struct ShellEnv(UnsafeCell<Option<Vec<String>>>);
unsafe impl Sync for ShellEnv {}

static SHELL_ENV: ShellEnv = ShellEnv(UnsafeCell::new(None));

fn init_env() {
    unsafe {
        let env_slot = &mut *SHELL_ENV.0.get();
        if env_slot.is_none() {
            *env_slot = Some(Vec::new());
            if let Some(env) = env_slot.as_mut() {
                env.push(String::from("PATH=/bin:/usr/bin"));
                env.push(String::from("HOME=/home/user"));
                env.push(String::from("PWD=/"));
                env.push(String::from("SHELL=/bin/sash"));
                env.push(String::from("TERM=vt100"));
            }
        }
    }
}

fn get_env_val(name: &str) -> Option<String> {
    unsafe {
        let env_slot = &*SHELL_ENV.0.get();
        if let Some(ref env) = *env_slot {
            for entry in env.iter() {
                if let Some(val) = entry.strip_prefix(name) {
                    if val.starts_with('=') {
                        return Some(String::from(&val[1..]));
                    }
                }
            }
        }
    }
    None
}

fn set_env(name: &str, val: &str) {
    let entry = alloc::format!("{}={}", name, val);
    unsafe {
        let env_slot = &mut *SHELL_ENV.0.get();
        if let Some(ref mut env) = *env_slot {
            for i in 0..env.len() {
                if env[i].starts_with(name) && env[i].len() > name.len() && env[i].as_bytes()[name.len()] == b'=' {
                    env[i] = entry;
                    return;
                }
            }
            env.push(entry);
        }
    }
}

fn unset_env(name: &str) {
    unsafe {
        let env_slot = &mut *SHELL_ENV.0.get();
        if let Some(ref mut env) = *env_slot {
            env.retain(|e| !(e.starts_with(name) && e.len() > name.len() && e.as_bytes()[name.len()] == b'='));
        }
    }
}

fn expand_vars(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' {
            let mut varname = String::new();
            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '_' {
                    varname.push(next);
                    chars.next();
                } else { break; }
            }
            if !varname.is_empty() {
                if let Some(val) = get_env_val(&varname) {
                    result.push_str(&val);
                }
            } else {
                result.push('$');
            }
        } else {
            result.push(c);
        }
    }
    result
}

sarga_main!(user_main);

fn user_main() {
    init_env();
    io::write_all(1, b"Sarga Shell (sash) v0.2.0\n").ok();
    io::write_all(1, b"Type help for commands. Use pwd, cd, export, unset, and ai.\n").ok();
    let mut input_buffer = String::new();
    loop {
        let prompt_dir = get_env_val("PWD").unwrap_or_else(|| String::from("/"));
        io::write_all(1, b"sash[").ok();
        io::write_all(1, prompt_dir.as_bytes()).ok();
        io::write_all(1, b"]> ").ok();
        let mut buf = [0u8; 1024];
        match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
                for c in s.chars() {
                    if c == '\n' || c == '\r' {
                        if !input_buffer.is_empty() {
                            let trimmed = input_buffer.trim().to_string();
                            if !trimmed.is_empty() {
                                execute_pipeline(&trimmed);
                            }
                            input_buffer.clear();
                        }
                    } else {
                        input_buffer.push(c);
                    }
                }
            }
            Err(_) => break,
        }
    }
}

fn execute_pipeline(line: &str) {
    let expanded = expand_vars(line);
    let commands = parse_pipeline(&expanded);
    if commands.is_empty() { return; }

    let n_cmds = commands.len();
    if n_cmds == 1 {
        execute_single(&commands[0]);
        return;
    }

    let mut prev_read: i64 = -1;
    let mut children = Vec::new();

    for (i, cmd_parts) in commands.iter().enumerate() {
        let mut pipe_write: i64 = -1;
        if i < n_cmds - 1 {
            let mut fds = [0i64; 2];
            let r = unsafe { libsarga::syscall::syscall1(22, fds.as_mut_ptr() as u64) };
            if r != 0 {
                io::write_all(1, b"sash: pipe failed\n").ok();
                return;
            }
            pipe_write = fds[1];
        }

        match libsarga::process::fork() {
            Ok(0) => {
                if prev_read >= 0 {
                    unsafe { libsarga::syscall::syscall2(33, prev_read as u64, 0) };
                    let _ = unsafe { libsarga::syscall::syscall1(3, prev_read as u64) };
                }
                if pipe_write >= 0 {
                    unsafe { libsarga::syscall::syscall2(33, pipe_write as u64, 1) };
                    let _ = unsafe { libsarga::syscall::syscall1(3, pipe_write as u64) };
                }
                exec_simple(cmd_parts);
                libsarga::process::exit(1);
            }
            Ok(pid) => {
                if prev_read >= 0 {
                    let _ = unsafe { libsarga::syscall::syscall1(3, prev_read as u64) };
                }
                if pipe_write >= 0 {
                    let _ = unsafe { libsarga::syscall::syscall1(3, pipe_write as u64) };
                }
                children.push(pid);
                prev_read = -1;
            }
            Err(_) => {
                io::write_all(1, b"sash: fork failed\n").ok();
                return;
            }
        }
    }

    for pid in children {
        let _ = libsarga::process::wait(pid);
    }
}

fn parse_pipeline(cmd: &str) -> Vec<Vec<String>> {
    let mut commands = Vec::new();
    let mut current_cmd: Vec<String> = Vec::new();
    let chars: Vec<char> = cmd.chars().collect();
    let mut pos = 0;
    while pos < chars.len() {
        while pos < chars.len() && (chars[pos] == ' ' || chars[pos] == '\t') { pos += 1; }
        if pos >= chars.len() { break; }
        if chars[pos] == '|' {
            commands.push(current_cmd);
            current_cmd = Vec::new();
            pos += 1;
            continue;
        }
        if chars[pos] == '>' {
            let append = pos + 1 < chars.len() && chars[pos + 1] == '>';
            if append { pos += 2; } else { pos += 1; }
            while pos < chars.len() && (chars[pos] == ' ' || chars[pos] == '\t') { pos += 1; }
            let mut file = String::new();
            while pos < chars.len() && chars[pos] != ' ' && chars[pos] != '\t' && chars[pos] != '|' && chars[pos] != '>' && chars[pos] != '<' {
                file.push(chars[pos]); pos += 1;
            }
            if append { current_cmd.push(String::from(">>")); }
            else { current_cmd.push(String::from(">")); }
            current_cmd.push(file);
            continue;
        }
        if chars[pos] == '<' {
            pos += 1;
            while pos < chars.len() && (chars[pos] == ' ' || chars[pos] == '\t') { pos += 1; }
            let mut file = String::new();
            while pos < chars.len() && chars[pos] != ' ' && chars[pos] != '\t' && chars[pos] != '|' && chars[pos] != '>' && chars[pos] != '<' {
                file.push(chars[pos]); pos += 1;
            }
            current_cmd.push(String::from("<"));
            current_cmd.push(file);
            continue;
        }
        let mut in_single = false;
        let mut in_double = false;
        let mut token = String::new();
        while pos < chars.len() {
            let c = chars[pos];
            if in_single {
                if c == '\'' { pos += 1; break; }
                token.push(c); pos += 1;
            } else if in_double {
                if c == '"' { pos += 1; break; }
                token.push(c); pos += 1;
            } else {
                if c == ' ' || c == '\t' || c == '|' || c == '>' || c == '<' { break; }
                if c == '\'' { in_single = true; pos += 1; }
                else if c == '"' { in_double = true; pos += 1; }
                else { token.push(c); pos += 1; }
            }
        }
        if !token.is_empty() { current_cmd.push(token); }
    }
    if !current_cmd.is_empty() { commands.push(current_cmd); }
    commands
}

fn execute_single(parts: &[String]) {
    if parts.is_empty() { return; }
    let cmd = &parts[0];
    match cmd.as_str() {
        "exit" => libsarga::process::exit(0),
        "help" => {
            io::write_all(1, b"Sarga Shell (sash) Commands:\n").ok();
            io::write_all(1, b"  help       - Show this help\n").ok();
            io::write_all(1, b"  exit       - Exit the shell\n").ok();
            io::write_all(1, b"  cd <dir>   - Change directory\n").ok();
            io::write_all(1, b"  pwd        - Print current directory\n").ok();
            io::write_all(1, b"  export     - Set env var (name=value)\n").ok();
            io::write_all(1, b"  unset      - Unset env var\n").ok();
            io::write_all(1, b"  env        - List environment\n").ok();
            io::write_all(1, b"  date       - Print current date and time\n").ok();
            io::write_all(1, b"  ai <int>   - Query VahiAI\n").ok();
            io::write_all(1, b"  [cmd]      - Execute system command\n").ok();
            io::write_all(1, b"  Pipes:     cmd1 | cmd2\n").ok();
            io::write_all(1, b"  Redir:     cmd > file, cmd >> file, cmd < file\n").ok();
        }
        "cd" => {
            let dir = if parts.len() > 1 { &parts[1] } else { "/" };
            let r = unsafe { libsarga::syscall::syscall1(80, dir.as_ptr() as u64) };
            if r != 0 {
                io::write_all(1, b"cd: ").ok();
                io::write_all(1, dir.as_bytes()).ok();
                io::write_all(1, b": no such directory\n").ok();
            } else {
                set_env("PWD", dir);
            }
        }
        "pwd" => {
            let cwd = get_env_val("PWD").unwrap_or_else(|| String::from("/"));
            io::write_all(1, cwd.as_bytes()).ok();
            io::write_all(1, b"\n").ok();
        }
        "export" => {
            if parts.len() < 2 {
                unsafe {
                    if let Some(ref env) = *SHELL_ENV.0.get() {
                        for e in env.iter() { println!("{}", e); }
                    }
                }
                return;
            }
            for i in 1..parts.len() {
                if let Some(idx) = parts[i].find('=') {
                    let name = &parts[i][..idx];
                    let val = &parts[i][idx+1..];
                    set_env(name, val);
                }
            }
        }
        "unset" => {
            for i in 1..parts.len() { unset_env(&parts[i]); }
        }
        "env" => {
            unsafe {
                if let Some(ref env) = *SHELL_ENV.0.get() {
                    for e in env.iter() { println!("{}", e); }
                }
            }
        }
        "ai" => {
            if parts.len() < 2 {
                io::write_all(1, b"Usage: ai <intent> [args...]\n").ok();
                return;
            }
            match libsarga::ai::query(&parts[1]) {
                Ok(resp) => {
                    io::write_all(1, b"VahiAI: ").ok();
                    io::write_all(1, resp.as_bytes()).ok();
                    io::write_all(1, b"\n").ok();
                }
                Err(_) => {
                    io::write_all(1, b"VahiAI: Error\n").ok();
                }
            }
        }
        _ => {
            match libsarga::process::fork() {
                Ok(0) => {
                    let args: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
                    let env_list = unsafe { (&*SHELL_ENV.0.get()).clone().unwrap_or_default() };
                    let env_refs: Vec<&str> = env_list.iter().map(|s| s.as_str()).collect();
                    libsarga::process::execve(cmd, &args, &env_refs);
                    io::write_all(1, b"sash: command not found: ").ok();
                    io::write_all(1, cmd.as_bytes()).ok();
                    io::write_all(1, b"\n").ok();
                    libsarga::process::exit(1);
                }
                Ok(pid) => { let _ = libsarga::process::wait(pid); }
                Err(_) => { io::write_all(1, b"sash: fork failed\n").ok(); }
            }
        }
    }
}

fn exec_simple(parts: &[String]) {
    if parts.is_empty() { libsarga::process::exit(0); }
    let cmd = &parts[0];
    let args: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
    let env_list = unsafe { (&*SHELL_ENV.0.get()).clone().unwrap_or_default() };
    let env_refs: Vec<&str> = env_list.iter().map(|s| s.as_str()).collect();
    let r = libsarga::process::execve(cmd, &args, &env_refs);
    if r < 0 {
        io::write_all(1, b"sash: command not found: ").ok();
        io::write_all(1, cmd.as_bytes()).ok();
        io::write_all(1, b"\n").ok();
    }
    libsarga::process::exit(1);
}
