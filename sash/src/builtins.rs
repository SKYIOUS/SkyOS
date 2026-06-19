use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::ffi::CString;
use alloc::format;
use libsarga::println;

pub fn try_builtin(cmd: &str, args: &[String]) -> Option<i64> {
    let cmd = match cmd.rsplit('/').next() { Some(c) => c, None => cmd };

    match cmd {
        "cd" => Some(builtin_cd(args)),
        "pwd" => Some(builtin_pwd()),
        "exit" => {
            let code = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            crate::save_history_on_exit();
            libsarga::process::exit(code);
        }
        "export" => Some(builtin_export(args)),
        "unset" => Some(builtin_unset(args)),
        "env" => Some(builtin_env()),
        "alias" => Some(builtin_alias(args)),
        "unalias" => Some(builtin_unalias(args)),
        "source" | "." => Some(builtin_source(args)),
        "exec" => Some(builtin_exec(args)),
        "true" => Some(0),
        "false" => Some(1),
        "test" | "[" => Some(builtin_test(args)),
        "read" => Some(builtin_read(args)),
        "printf" => Some(builtin_printf(args)),
        "echo" => Some(builtin_echo(args)),
        "jobs" => Some(builtin_jobs()),
        "fg" => Some(builtin_fg(args)),
        "bg" => Some(builtin_bg(args)),
        "history" => Some(builtin_history(args)),
        "type" => Some(builtin_type(args)),
        "help" => Some(builtin_help()),
        "ai" => Some(builtin_ai(args)),
        "wait" => Some(builtin_wait(args)),
        "eval" => Some(builtin_eval(args)),
        "shift" => Some(builtin_shift(args)),
        _ => None,
    }
}

fn builtin_cd(args: &[String]) -> i64 {
    let old = crate::get_env("PWD").unwrap_or_default();
    let target = if args.len() > 1 {
        if args[1] == "-" {
            crate::get_env("OLDPWD").unwrap_or_else(|| String::from("/"))
        } else {
            args[1].clone()
        }
    } else {
        crate::get_env("HOME").unwrap_or_else(|| String::from("/"))
    };
    let c_str = match CString::new(target.as_bytes()) {
        Ok(c) => c,
        Err(_) => return 1,
    };
    let r = unsafe { libsarga::syscall::syscall1(80, c_str.as_ptr() as u64) };
    if r == 0 {
        crate::set_env("OLDPWD", &old);
        crate::set_env("PWD", &target);
    } else {
        println!("cd: {}: {}", target, format_errno(r));
    }
    r as i64
}

fn builtin_pwd() -> i64 {
    let mut buf = [0u8; 4096];
    let r = unsafe { libsarga::syscall::syscall2(79, buf.as_mut_ptr() as u64, 4095u64) };
    if r > 0 {
        let len = r as usize;
        buf[len] = 0;
        if let Ok(s) = core::ffi::CStr::from_bytes_until_nul(&buf[..len+1]) {
            println!("{}", s.to_str().unwrap_or(""));
        }
    }
    0
}

fn builtin_export(args: &[String]) -> i64 {
    for arg in &args[1..] {
        let mut parts = arg.splitn(2, '=');
        let name = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("");
        crate::set_env(name, val);
    }
    0
}

fn builtin_unset(args: &[String]) -> i64 {
    for arg in &args[1..] {
        crate::unset_env(arg);
    }
    0
}

fn builtin_env() -> i64 {
    for (k, v) in crate::get_env_all() {
        println!("{}={}", k, v);
    }
    0
}

fn builtin_alias(args: &[String]) -> i64 {
    if args.len() == 1 {
        for (k, v) in crate::get_aliases() {
            println!("alias {}='{}'", k, v);
        }
        return 0;
    }
    for arg in &args[1..] {
        let mut parts = arg.splitn(2, '=');
        let name = parts.next().unwrap_or("");
        if let Some(val) = parts.next() {
            crate::set_alias(name, val);
        }
    }
    0
}

fn builtin_unalias(args: &[String]) -> i64 {
    for arg in &args[1..] {
        if arg == "-a" { crate::clear_aliases(); return 0; }
        crate::remove_alias(arg);
    }
    0
}

fn builtin_source(args: &[String]) -> i64 {
    if args.len() < 2 { println!("source: missing filename"); return 1; }
    let script_args: Vec<String> = args[2..].to_vec();
    crate::scripting::run_script_with_args(&args[1], &script_args)
}

fn builtin_exec(args: &[String]) -> i64 {
    if args.len() < 2 { return 1; }
    let args_refs: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
    let env_strings = crate::get_env_refs();
    let env_refs: Vec<&str> = env_strings.iter().map(|s| s.as_str()).collect();
    let r = libsarga::process::execve(&args[1], &args_refs, &env_refs);
    println!("exec: {}: failed", args[1]);
    r as i64
}

fn builtin_echo(args: &[String]) -> i64 {
    let newline = !args.get(1).map_or(false, |a| a == "-n");
    let start = if !newline && args.len() > 1 { 2 } else { 1 };
    let mut s = String::new();
    for (i, a) in args[start..].iter().enumerate() {
        if i > 0 { s.push(' '); }
        s.push_str(a);
    }
    if newline { s.push('\n'); }
    libsarga::print!("{}", s);
    0
}

fn builtin_test(args: &[String]) -> i64 {
    let test_args = if args[0] == "[" && args.last().map_or(false, |a| a == "]") {
        &args[1..args.len()-1]
    } else {
        &args[1..]
    };
    if test_args.is_empty() { return 1; }

    match test_args[0].as_str() {
        "-f" => test_file(test_args.get(1), |_| true),
        "-d" => test_file(test_args.get(1), is_dir),
        "-e" => test_file(test_args.get(1), |_| true),
        "-z" => test_str_empty(test_args.get(1)),
        "-n" => test_str_not_empty(test_args.get(1)),
        "-x" => test_file(test_args.get(1), |p| is_file(p) && is_executable(p)),
        "-w" => test_file(test_args.get(1), |_| true),
        "-r" => test_file(test_args.get(1), |_| true),
        "=" | "==" => {
            let empty = String::new();
            let left = test_args.get(1).unwrap_or(&empty);
            let right = test_args.get(2).unwrap_or(&empty);
            if left == right { 0 } else { 1 }
        }
        "!=" => {
            let empty = String::new();
            let left = test_args.get(1).unwrap_or(&empty);
            let right = test_args.get(2).unwrap_or(&empty);
            if left != right { 0 } else { 1 }
        }
        "-eq" => test_num(test_args.get(1), test_args.get(2), |a,b| a == b),
        "-ne" => test_num(test_args.get(1), test_args.get(2), |a,b| a != b),
        "-lt" => test_num(test_args.get(1), test_args.get(2), |a,b| a < b),
        "-le" => test_num(test_args.get(1), test_args.get(2), |a,b| a <= b),
        "-gt" => test_num(test_args.get(1), test_args.get(2), |a,b| a > b),
        "-ge" => test_num(test_args.get(1), test_args.get(2), |a,b| a >= b),
        "!" => if builtin_test(&[String::from("test"), test_args[1].clone()]) == 0 { 1 } else { 0 },
        _ => 1,
    }
}

fn test_file(arg: Option<&String>, check: fn(&str) -> bool) -> i64 {
    match arg {
        Some(p) if check(p) => 0,
        _ => 1,
    }
}

fn test_str_empty(arg: Option<&String>) -> i64 {
    match arg {
        Some(s) if s.is_empty() => 0,
        Some(_) => 1,
        None => 0,
    }
}

fn test_str_not_empty(arg: Option<&String>) -> i64 {
    match arg {
        Some(s) if !s.is_empty() => 0,
        _ => 1,
    }
}

fn test_num(a: Option<&String>, b: Option<&String>, cmp: fn(i64, i64) -> bool) -> i64 {
    let a_val: i64 = a.and_then(|s| s.parse().ok()).unwrap_or(0);
    let b_val: i64 = b.and_then(|s| s.parse().ok()).unwrap_or(0);
    if cmp(a_val, b_val) { 0 } else { 1 }
}

fn is_dir(path: &str) -> bool {
    let c_str = match CString::new(path.as_bytes()) { Ok(c) => c, Err(_) => return false };
    let mut st = [0i64; 13];
    let r = unsafe { libsarga::syscall::syscall2(4, c_str.as_ptr() as u64, st.as_mut_ptr() as u64) };
    r == 0 && (st[3] & 0o40000) != 0
}

fn is_file(path: &str) -> bool {
    let c_str = match CString::new(path.as_bytes()) { Ok(c) => c, Err(_) => return false };
    let mut st = [0i64; 13];
    let r = unsafe { libsarga::syscall::syscall2(4, c_str.as_ptr() as u64, st.as_mut_ptr() as u64) };
    r == 0
}

fn is_executable(path: &str) -> bool {
    let c_str = match CString::new(path.as_bytes()) { Ok(c) => c, Err(_) => return false };
    let mut st = [0i64; 13];
    let r = unsafe { libsarga::syscall::syscall2(4, c_str.as_ptr() as u64, st.as_mut_ptr() as u64) };
    r == 0 && (st[3] & 0o111) != 0
}

fn builtin_read(args: &[String]) -> i64 {
    let var = args.get(1).map(|s| s.clone()).unwrap_or_else(|| String::from("REPLY"));
    let mut buf = [0u8; 4096];
    let n = libsarga::io::read(0, &mut buf).unwrap_or(0);
    if n == 0 { return 1; }
    let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
    let trimmed = s.trim_end_matches('\n');
    crate::set_env(&var, trimmed);
    0
}

fn builtin_printf(args: &[String]) -> i64 {
    let fmt = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let output = if fmt == "%s" || fmt == "%s\n" {
        let arg = args.get(2).map(|s| s.as_str()).unwrap_or("");
        format!("{}\n", arg)
    } else {
        let mut s = fmt.to_string();
        if !s.ends_with('\n') { s.push('\n'); }
        s
    };
    libsarga::print!("{}", output);
    0
}

fn builtin_jobs() -> i64 {
    crate::print_jobs();
    0
}

fn builtin_fg(args: &[String]) -> i64 {
    let job_id = args.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
    crate::fg_job(job_id)
}

fn builtin_bg(args: &[String]) -> i64 {
    let job_id = args.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
    crate::bg_job(job_id)
}

fn builtin_history(args: &[String]) -> i64 {
    let n = args.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(usize::MAX);
    crate::print_history(n);
    0
}

fn builtin_type(args: &[String]) -> i64 {
    let cmd = match args.get(1) {
        Some(c) => c.as_str(),
        None => { println!("type: missing argument"); return 1; }
    };
    if let Some(alias) = crate::get_alias(cmd) { println!("{} is aliased to `{}'", cmd, alias); return 0; }
    if matches_builtin(cmd) { println!("{} is a shell builtin", cmd); return 0; }
    if let Some(path) = find_in_path(cmd) { println!("{} is {}", cmd, path); return 0; }
    println!("{}: not found", cmd);
    1
}

pub fn matches_builtin(cmd: &str) -> bool {
    matches!(cmd, "cd"|"pwd"|"exit"|"export"|"unset"|"env"|"alias"|"unalias"
        |"source"|"."|"exec"|"true"|"false"|"test"|"["|"read"|"printf"
        |"echo"|"jobs"|"fg"|"bg"|"history"|"type"|"help"|"ai"|"wait"|"eval"|"shift")
}

pub fn find_in_path(cmd: &str) -> Option<String> {
    let path = crate::get_env("PATH").unwrap_or_else(|| String::from("/bin"));
    for dir in path.split(':') {
        let full: String = if dir.ends_with('/') { format!("{}{}", dir, cmd) } else { format!("{}/{}", dir, cmd) };
        let c_str = match CString::new(full.as_bytes()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut st = [0i64; 13];
        let r = unsafe { libsarga::syscall::syscall2(4, c_str.as_ptr() as u64, st.as_mut_ptr() as u64) };
        if r == 0 { return Some(full); }
    }
    None
}

fn builtin_help() -> i64 {
    println!("Sarga Shell (sash) Commands:");
    println!("  help       - Show this help");
    println!("  exit       - Exit the shell");
    println!("  cd <dir>   - Change directory");
    println!("  pwd        - Print current directory");
    println!("  export     - Set env var (name=value)");
    println!("  unset      - Unset env var");
    println!("  env        - List environment");
    println!("  echo       - Print arguments");
    println!("  source     - Execute script");
    println!("  exec       - Replace shell with command");
    println!("  alias      - Manage aliases");
    println!("  test/ [    - Test conditions");
    println!("  read       - Read from stdin");
    println!("  printf     - Format and print");
    println!("  jobs       - List background jobs");
    println!("  fg/bg      - Foreground/background job");
    println!("  history    - Command history");
    println!("  type       - Show command type");
    println!("  true/false - Return status");
    println!("  Pipes:     cmd1 | cmd2");
    println!("  Redir:     cmd > file, cmd >> file, cmd < file, cmd 2> file");
    println!("  Chains:    cmd1 && cmd2, cmd1 || cmd2, cmd1; cmd2");
    println!("  Background: cmd &");
    println!("  Scripting: if/for/while/until/case/function");
    0
}

fn builtin_ai(args: &[String]) -> i64 {
    if args.len() < 2 { println!("Usage: ai <intent> [args...]"); return 1; }
    match libsarga::ai::query(&args[1]) {
        Ok(resp) => { println!("SARGAAI: {}", resp); 0 }
        Err(_) => { println!("SARGAAI: Error"); 1 }
    }
}

fn builtin_wait(args: &[String]) -> i64 {
    if args.len() > 1 {
        if let Ok(pid) = args[1].parse::<u64>() {
            libsarga::process::wait(pid).unwrap_or(0) as i64
        } else {
            println!("wait: invalid pid: {}", args[1]); 1
        }
    } else {
        // Wait for all children
        loop {
            let r = unsafe { libsarga::syscall::syscall3(61, -1i64 as u64, 0u64, 0u64) };
            if r < 0 { break; }
        }
        0
    }
}

fn builtin_eval(args: &[String]) -> i64 {
    let expr = args[1..].join(" ");
    let tokens = match crate::parser::tokenize(&expr) {
        Ok(t) => t,
        Err(e) => { println!("eval: {}", e); return 1; }
    };
    let pipelines = crate::parser::parse(&tokens);
    crate::executor::execute_pipelines(pipelines)
}

fn builtin_shift(args: &[String]) -> i64 {
    let n = args.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
    // shift is a no-op without scripting context, but we store for script use
    if n > 0 { crate::shift_positional(n) }
    0
}

fn format_errno(r: i64) -> &'static str {
    match -r {
        2 => "No such file or directory",
        13 => "Permission denied",
        20 => "Not a directory",
        _ => "error",
    }
}
