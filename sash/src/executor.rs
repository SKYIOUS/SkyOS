use crate::parser::{Command, Pipeline, Connector, RedirectKind};
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::ffi::CString;
use alloc::format;
use libsarga::println;

pub fn execute_pipelines(pipelines: Vec<Pipeline>) -> i64 {
    let mut last_exit = 0i64;
    for pipeline in pipelines {
        let mut should_run = true;
        if let Some(ref conn) = pipeline.connector {
            match conn {
                Connector::And => should_run = last_exit == 0,
                Connector::Or => should_run = last_exit != 0,
                _ => {}
            }
        }
        if !should_run { continue; }

        let bg = pipeline.commands.last().map(|c| c.background).unwrap_or(false);

        if pipeline.commands.len() == 1 && !bg {
            last_exit = execute_command(&pipeline.commands[0], None, None, false);
        } else {
            last_exit = execute_pipeline(&pipeline.commands, bg);
        }
    }
    last_exit
}

fn execute_command(cmd: &Command, stdin: Option<i64>, stdout: Option<i64>, bg: bool) -> i64 {
    if cmd.args.is_empty() { return 0; }

    let cmd_name = &cmd.args[0];
    if let Some(exit_code) = crate::builtins::try_builtin(cmd_name, &cmd.args) {
        return exit_code;
    }

    // Check if it's a shell function
    if let Some(body) = crate::get_function(cmd_name) {
        let mut expanded = body.join("\n");
        // Replace $1, $2, ... $@, $* with arguments
        for (i, arg) in cmd.args[1..].iter().enumerate() {
            let pattern = alloc::format!("${}", i + 1);
            expanded = expanded.replace(&pattern, arg);
            let pattern_brace = alloc::format!("${{{}}}", i + 1);
            expanded = expanded.replace(&pattern_brace, arg);
        }
        let args_joined = cmd.args[1..].join(" ");
        expanded = expanded.replace("$@", &args_joined);
        expanded = expanded.replace("$*", &args_joined);
        expanded = expanded.replace("$#", &alloc::format!("{}", cmd.args.len() - 1));
        let tokens = crate::parser::tokenize(&expanded).unwrap_or_default();
        let pipelines = crate::parser::parse(&tokens);
        return crate::executor::execute_pipelines(pipelines);
    }

    // Glob expansion
    let expanded_args = glob_expand_args(&cmd.args);

    let pid = match libsarga::process::fork() {
        Ok(p) => p,
        Err(_) => { println!("sash: fork failed"); return -1; }
    };

    if pid == 0 {
        // Child process
        apply_redirections(cmd);
        if let Some(fd) = stdin { unsafe { libsarga::syscall::syscall2(33, fd as u64, 0) }; }
        if let Some(fd) = stdout { unsafe { libsarga::syscall::syscall2(33, fd as u64, 1) }; }

        let args: Vec<&str> = expanded_args.iter().map(|s| s.as_str()).collect();
        let env_strings = crate::get_env_refs();
        let env_refs: Vec<&str> = env_strings.iter().map(|s| s.as_str()).collect();
        let r = libsarga::process::execve(cmd_name, &args, &env_refs);
        if r < 0 {
            println!("sash: command not found: {}", cmd_name);
        }
        libsarga::process::exit(127);
    }

    if !bg {
        libsarga::process::wait(pid).unwrap_or(0) as i64
    } else {
        crate::add_job(pid, cmd);
        0
    }
}

fn has_glob_chars(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

fn glob_expand_args(args: &[String]) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for arg in args {
        if has_glob_chars(arg) {
            let matches = do_glob(arg);
            if matches.is_empty() {
                result.push(arg.clone());
            } else {
                for m in matches {
                    result.push(m);
                }
            }
        } else {
            result.push(arg.clone());
        }
    }
    result
}

fn do_glob(pattern: &str) -> Vec<String> {
    // Determine directory part and pattern part
    let (dir, pat) = if let Some(pos) = pattern.rfind('/') {
        let d = &pattern[..pos];
        let p = &pattern[pos+1..];
        (if d.is_empty() { "/" } else { d }, p)
    } else {
        (".", pattern)
    };

    let dir_cstr = match CString::new(dir.as_bytes()) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let fd = unsafe { libsarga::syscall::syscall2(257, dir_cstr.as_ptr() as u64, 0x100000u64) };
    if fd < 0 { return Vec::new(); }

    let mut matches: Vec<String> = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { libsarga::syscall::syscall3(217, fd as u64, buf.as_mut_ptr() as u64, 4096u64) };
        if n <= 0 { break; }
        let mut offset = 0;
        while offset < n as usize {
            let reclen_bytes = &buf[offset+16..offset+18];
            let reclen = u16::from_ne_bytes([reclen_bytes[0], reclen_bytes[1]]) as usize;
            let namelen_bytes = &buf[offset+18..offset+20];
            let namelen = u16::from_ne_bytes([namelen_bytes[0], namelen_bytes[1]]) as usize;
            let name = core::str::from_utf8(&buf[offset+20..offset+20+namelen]).unwrap_or("");
            if name != "." && name != ".." && glob_match(pat, name) {
                let full = if dir == "." { name.to_string() } else { format!("{}/{}", dir, name) };
                matches.push(full);
            }
            offset += reclen;
            if reclen == 0 { break; }
        }
    }
    let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
    matches.sort();
    matches
}

fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    glob_match_inner(&p, &n, 0, 0)
}

fn glob_match_inner(p: &[char], n: &[char], pi: usize, ni: usize) -> bool {
    if pi >= p.len() { return ni >= n.len(); }
    match p[pi] {
        '*' => {
            if pi + 1 >= p.len() { return true; }
            let mut j = ni;
            while j <= n.len() {
                if glob_match_inner(p, n, pi + 1, j) { return true; }
                j += 1;
            }
            false
        }
        '?' => ni < n.len() && glob_match_inner(p, n, pi + 1, ni + 1),
        '[' => {
            // Character class [...]
            if ni >= n.len() { return false; }
            let c = n[ni];
            let mut j = pi + 1;
            if j >= p.len() { return false; }
            let negate = if p[j] == '!' { j += 1; true } else { false };
            let mut matched = false;
            while j < p.len() && p[j] != ']' {
                if j + 2 < p.len() && p[j+1] == '-' && p[j+2] != ']' {
                    if c >= p[j] && c <= p[j+2] { matched = true; }
                    j += 3;
                } else {
                    if c == p[j] { matched = true; }
                    j += 1;
                }
            }
            if negate { matched = !matched; }
            if !matched { return false; }
            // Skip to after ]
            while j < p.len() && p[j] != ']' { j += 1; }
            glob_match_inner(p, n, j + 1, ni + 1)
        }
        c => ni < n.len() && n[ni] == c && glob_match_inner(p, n, pi + 1, ni + 1),
    }
}

fn execute_pipeline(commands: &[Command], bg: bool) -> i64 {
    let n = commands.len();
    if n == 0 { return 0; }
    if n == 1 { return execute_command(&commands[0], None, None, bg); }

    let mut prev_read: Option<i64> = None;
    let mut children: Vec<u64> = Vec::new();

    for (i, cmd) in commands.iter().enumerate() {
        let mut pipe_write: Option<i64> = None;

        if i < n - 1 {
            let mut fds = [0i64; 2];
            let r = unsafe { libsarga::syscall::syscall1(22, fds.as_mut_ptr() as u64) };
            if r != 0 { println!("sash: pipe failed"); return -1; }
            pipe_write = Some(fds[1]);
        }

        let stdin = prev_read;
        let stdout = pipe_write;
    let expanded_args = glob_expand_args(&cmd.args);
    let expanded_name = expanded_args[0].clone();

    let pid = match libsarga::process::fork() {
            Ok(p) => p,
            Err(_) => { println!("sash: fork failed"); return -1; }
        };

        if pid == 0 {
            apply_redirections(cmd);
            if let Some(fd) = stdin {
                unsafe { libsarga::syscall::syscall2(33, fd as u64, 0); }
                let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
            }
            if let Some(fd) = stdout {
                unsafe { libsarga::syscall::syscall2(33, fd as u64, 1); }
                let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
            }

            let args: Vec<&str> = expanded_args.iter().map(|s| s.as_str()).collect();
            let env_strings = crate::get_env_refs();
            let env_refs: Vec<&str> = env_strings.iter().map(|s| s.as_str()).collect();
            let r = libsarga::process::execve(&expanded_name, &args, &env_refs);
            if r < 0 {
                println!("sash: command not found: {}", expanded_name);
            }
            libsarga::process::exit(127);
        }

        if let Some(fd) = prev_read { let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) }; }
        if let Some(fd) = pipe_write { let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) }; }

        children.push(pid);
        prev_read = if i < n - 1 { stdin } else { None };
    }

    if bg {
        if let Some(cmd) = commands.last() {
            crate::add_job(children[children.len() - 1], cmd);
        }
        return 0;
    }

    let mut last_exit = 0;
    for pid in children {
        last_exit = libsarga::process::wait(pid).unwrap_or(0) as i64;
    }
    last_exit
}

fn apply_redirections(cmd: &Command) {
    for redir in &cmd.redirects {
        match redir.kind {
            RedirectKind::Out | RedirectKind::Append => {
                let flags = if redir.kind == RedirectKind::Append { 0x401 } else { 0x241 };
                if let Ok(fd) = crate::open_file(&redir.file, flags) {
                    unsafe { libsarga::syscall::syscall2(33, fd as u64, 1) };
                    let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
                }
            }
            RedirectKind::In => {
                if let Ok(fd) = crate::open_file(&redir.file, 0) {
                    unsafe { libsarga::syscall::syscall2(33, fd as u64, 0) };
                    let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
                }
            }
            RedirectKind::Stderr => {
                if let Ok(fd) = crate::open_file(&redir.file, 0x241) {
                    unsafe { libsarga::syscall::syscall2(33, fd as u64, 2) };
                    let _ = unsafe { libsarga::syscall::syscall1(3, fd as u64) };
                }
            }
        }
    }
}

pub fn capture_output(cmd_str: &str) -> String {
    let tokens = crate::parser::tokenize(cmd_str).unwrap_or_default();
    let pipelines = crate::parser::parse(&tokens);
    if pipelines.is_empty() || pipelines[0].commands.is_empty() { return String::new(); }

    let mut fds = [0i64; 2];
    if unsafe { libsarga::syscall::syscall1(22, fds.as_mut_ptr() as u64) } != 0 {
        return String::new();
    }

    let pid = match libsarga::process::fork() {
        Ok(p) => p,
        Err(_) => { return String::new(); }
    };

    if pid == 0 {
        let _ = unsafe { libsarga::syscall::syscall1(3, fds[0] as u64) };
        unsafe { libsarga::syscall::syscall2(33, fds[1] as u64, 1) };
        let cmd = &pipelines[0].commands[0];
        let expanded_args = glob_expand_args(&cmd.args);
        let args: Vec<&str> = expanded_args.iter().map(|s| s.as_str()).collect();
        let env_strings = crate::get_env_refs();
        let env_refs: Vec<&str> = env_strings.iter().map(|s| s.as_str()).collect();
        let _ = libsarga::process::execve(&expanded_args[0], &args, &env_refs);
        libsarga::process::exit(1);
    }

    let _ = unsafe { libsarga::syscall::syscall1(3, fds[1] as u64) };
    let mut buf = [0u8; 4096];
    let mut out = String::new();
    loop {
        let n = libsarga::io::read(fds[0], &mut buf).unwrap_or(0);
        if n == 0 { break; }
        out.push_str(core::str::from_utf8(&buf[..n]).unwrap_or(""));
    }
    let _ = unsafe { libsarga::syscall::syscall1(3, fds[0] as u64) };
    libsarga::process::wait(pid).ok();
    out
}
