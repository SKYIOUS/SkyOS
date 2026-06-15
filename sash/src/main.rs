#![no_std]
#![no_main]
use libsarga::sarga_main;
use libsarga::println;
extern crate alloc;
use core::cell::UnsafeCell;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::ffi::CString;

mod parser;
mod executor;
mod builtins;
mod readline;
mod scripting;

struct ShellEnv(UnsafeCell<Option<Vec<String>>>);
unsafe impl Sync for ShellEnv {}

static SHELL_ENV: ShellEnv = ShellEnv(UnsafeCell::new(None));

struct AliasTable(UnsafeCell<Vec<(String, String)>>);
unsafe impl Sync for AliasTable {}
static ALIAS_TABLE: AliasTable = AliasTable(UnsafeCell::new(Vec::new()));

struct JobEntry {
    pid: u64,
    cmd: String,
}
struct JobTable(UnsafeCell<Vec<JobEntry>>);
unsafe impl Sync for JobTable {}
static JOB_TABLE: JobTable = JobTable(UnsafeCell::new(Vec::new()));

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

pub fn get_env(name: &str) -> Option<String> {
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

pub fn set_env(name: &str, val: &str) {
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

pub fn unset_env(name: &str) {
    unsafe {
        let env_slot = &mut *SHELL_ENV.0.get();
        if let Some(ref mut env) = *env_slot {
            env.retain(|e| !(e.starts_with(name) && e.len() > name.len() && e.as_bytes()[name.len()] == b'='));
        }
    }
}

pub fn get_env_all() -> Vec<(String, String)> {
    let mut out = Vec::new();
    unsafe {
        if let Some(ref env) = *SHELL_ENV.0.get() {
            for e in env.iter() {
                if let Some(idx) = e.find('=') {
                    out.push((e[..idx].to_string(), e[idx+1..].to_string()));
                }
            }
        }
    }
    out
}

pub fn get_env_refs() -> Vec<String> {
    unsafe {
        let env_slot = &*SHELL_ENV.0.get();
        if let Some(ref env) = *env_slot {
            env.clone()
        } else {
            Vec::new()
        }
    }
}

pub fn set_alias(name: &str, val: &str) {
    unsafe {
        let tbl = &mut *ALIAS_TABLE.0.get();
        for (k, v) in tbl.iter_mut() {
            if k == name { *v = val.to_string(); return; }
        }
        tbl.push((name.to_string(), val.to_string()));
    }
}

pub fn remove_alias(name: &str) {
    unsafe {
        let tbl = &mut *ALIAS_TABLE.0.get();
        tbl.retain(|(k, _)| k != name);
    }
}

pub fn clear_aliases() {
    unsafe { (&mut *ALIAS_TABLE.0.get()).clear(); }
}

pub fn get_aliases() -> Vec<(String, String)> {
    unsafe { (&*ALIAS_TABLE.0.get()).clone() }
}

pub fn get_alias(name: &str) -> Option<String> {
    unsafe {
        let tbl = &*ALIAS_TABLE.0.get();
        for (k, v) in tbl.iter() {
            if k == name { return Some(v.clone()); }
        }
    }
    None
}

pub fn add_job(pid: u64, cmd: &parser::Command) {
    let cmd_str = cmd.args.join(" ");
    unsafe {
        let tbl = &mut *JOB_TABLE.0.get();
        tbl.push(JobEntry { pid, cmd: cmd_str });
    }
}

pub fn print_jobs() {
    unsafe {
        let tbl = &*JOB_TABLE.0.get();
        for (i, job) in tbl.iter().enumerate() {
            println!("[{}] {} {}", i + 1, job.pid, job.cmd);
        }
    }
}

pub fn fg_job(id: usize) -> i64 {
    unsafe {
        let tbl = &mut *JOB_TABLE.0.get();
        if id == 0 || id > tbl.len() { println!("fg: job not found"); return 1; }
        let job = &tbl[id - 1];
        let code = libsarga::process::wait(job.pid).unwrap_or(1);
        tbl.remove(id - 1);
        code as i64
    }
}

pub fn bg_job(id: usize) -> i64 {
    unsafe {
        let tbl = &mut *JOB_TABLE.0.get();
        if id == 0 || id > tbl.len() { println!("bg: job not found"); return 1; }
        println!("[{}] {} &", id, tbl[id - 1].pid);
        0
    }
}

pub fn print_history(n: usize) {
    let history = unsafe { &mut *HISTORY.0.get() };
    if let Some(ref h) = *history {
        h.print(n);
    }
}

struct LastExit(UnsafeCell<i64>);
unsafe impl Sync for LastExit {}
static LAST_EXIT: LastExit = LastExit(UnsafeCell::new(0));

pub fn set_last_exit(code: i64) {
    unsafe { *LAST_EXIT.0.get() = code; }
}

pub fn get_last_exit() -> i64 {
    unsafe { *LAST_EXIT.0.get() }
}

struct FuncTable(UnsafeCell<Vec<(String, Vec<String>)>>);
unsafe impl Sync for FuncTable {}
static FUNC_TABLE: FuncTable = FuncTable(UnsafeCell::new(Vec::new()));

pub fn define_function(name: &str, body: &[String]) {
    unsafe {
        let tbl = &mut *FUNC_TABLE.0.get();
        for (k, v) in tbl.iter_mut() {
            if k == name { *v = body.to_vec(); return; }
        }
        tbl.push((name.to_string(), body.to_vec()));
    }
}

pub fn get_function(name: &str) -> Option<Vec<String>> {
    unsafe {
        let tbl = &*FUNC_TABLE.0.get();
        for (k, v) in tbl.iter() {
            if k == name { return Some(v.clone()); }
        }
    }
    None
}

struct HistoryCell(UnsafeCell<Option<readline::History>>);
unsafe impl Sync for HistoryCell {}
static HISTORY: HistoryCell = HistoryCell(UnsafeCell::new(None));

#[allow(unused_variables)]
pub fn shift_positional(n: usize) {
    // No-op for now — script-level positional shifting would need script context
}

pub fn save_history_on_exit() {
    unsafe {
        let h = &mut *HISTORY.0.get();
        if let Some(ref hist) = *h {
            hist.save();
        }
    }
}

pub fn open_file(path: &str, flags: u64) -> Result<i64, ()> {
    let c_str = CString::new(path.as_bytes()).map_err(|_| ())?;
    let fd = unsafe { libsarga::syscall::syscall2(2, c_str.as_ptr() as u64, flags) };
    if fd < 0 { Err(()) } else { Ok(fd) }
}

sarga_main!(user_main);

fn make_prompt() -> String {
    let ps1 = get_env("PS1").unwrap_or_else(|| String::from("sash[\\w]> "));
    let mut out = String::new();
    let chars: Vec<char> = ps1.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'w' => out.push_str(&get_env("PWD").unwrap_or_else(|| String::from("/"))),
                'u' => out.push_str(&get_env("USER").unwrap_or_else(|| String::from("user"))),
                'h' => out.push_str("sarga"),
                's' => out.push_str("sash"),
                'n' => out.push('\n'),
                '\\' => out.push('\\'),
                '$' => out.push(if get_last_exit() == 0 { '$' } else { '#' }),
                c => { out.push('\\'); out.push(c); }
            }
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn read_with_continuation(history: &mut readline::History, prompt: &str) -> String {
    let mut input = readline::read_line(history, prompt);
    // Check for backslash continuation (outside quotes)
    loop {
        let trimmed = input.trim_end_matches(&['\r', '\n'][..]);
        if trimmed.ends_with('\\') {
            // Check if the backslash is quoted (simple heuristic)
            let mut in_single = false;
            let mut in_double = false;
            let mut is_escaped = false;
            for c in trimmed.chars() {
                if is_escaped { is_escaped = false; continue; }
                match c {
                    '\\' => is_escaped = true,
                    '\'' if !in_double => in_single = !in_single,
                    '"' if !in_single => in_double = !in_double,
                    _ => {}
                }
            }
            if !in_single && !in_double && is_escaped {
                let cont = readline::read_line(history, "> ");
                input = input.trim_end_matches('\\').to_string() + &cont;
                continue;
            }
        }
        break;
    }
    input
}

fn user_main() {
    init_env();

    unsafe {
        let h = &mut *HISTORY.0.get();
        *h = Some(readline::History::new(1000));
    }

    println!("Sarga Shell (sash) v0.3.0");
    println!("Type help for commands.");

    loop {
        let prompt = make_prompt();

        let input;
        {
            let history = unsafe { &mut *HISTORY.0.get() };
            input = if let Some(ref mut h) = *history {
                read_with_continuation(h, &prompt)
            } else {
                read_raw_line()
            };
        }

        let trimmed = input.trim();
        if trimmed.is_empty() { continue; }

        // Alias expansion
        let expanded = if let Some(alias) = get_alias(trimmed.split_whitespace().next().unwrap_or("")) {
            let rest = trimmed.splitn(2, ' ').nth(1).unwrap_or("");
            if rest.is_empty() { alias } else { alloc::format!("{} {}", alias, rest) }
        } else {
            trimmed.to_string()
        };

        // Variable expansion (use global $?)
        let expanded = expand_shell_vars(&expanded);

        let tokens = match parser::tokenize(&expanded) {
            Ok(t) => t,
            Err(e) => { println!("sash: {}", e); continue; }
        };
        let pipelines = parser::parse(&tokens);
        let exit_code = executor::execute_pipelines(pipelines);
        set_last_exit(exit_code);
    }
}

fn expand_shell_vars(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0;
    while pos < chars.len() {
        if chars[pos] == '$' && pos + 1 < chars.len() {
            match chars[pos + 1] {
                '{' => {
                    if let Some(end) = s[pos+2..].find('}') {
                        let var = &s[pos+2..pos+2+end];
                        out.push_str(&get_env(var).unwrap_or_default());
                        pos += 3 + end;
                    } else { out.push(chars[pos]); pos += 1; }
                }
                '?' => { out.push_str(&alloc::format!("{}", get_last_exit())); pos += 2; }
                '$' => { pos += 2; }
                c if c.is_alphabetic() || c == '_' => {
                    let start = pos + 1;
                    let mut end = start;
                    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') { end += 1; }
                    out.push_str(&get_env(&s[start..end]).unwrap_or_default());
                    pos = end;
                }
                _ => { out.push(chars[pos]); pos += 1; }
            }
        } else {
            out.push(chars[pos]);
            pos += 1;
        }
    }
    out
}

fn read_raw_line() -> String {
    let mut buf = [0u8; 1024];
    let mut input = String::new();
    loop {
        match libsarga::io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                for &c in &buf[..n] {
                    if c == b'\n' || c == b'\r' { return input; }
                    if c == 0x7f || c == 0x08 { input.pop(); }
                    else { input.push(c as char); }
                }
            }
            Err(_) => break,
        }
    }
    input
}
