#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::collections::BTreeMap;
use alloc::ffi::CString;
use libsarga::sarga_main;
use libsarga::syscall::*;

const PATH_DIRS: &[&str] = &["/bin", "/sbin", "/usr/bin", "/usr/local/bin"];
const MAX_LINE: usize = 4096;
const PROMPT: &[u8] = b"skyos$ ";
const HISTFILE: &str = "/home/root/.sargash_history\0";
const HIST_MAX: usize = 500;

static mut ENV: BTreeMap<String, String> = BTreeMap::new();
static mut HISTORY: Vec<String> = Vec::new();
static mut HIST_POS: isize = -1;
static mut ALIASES: BTreeMap<String, String> = BTreeMap::new();
static mut FUNCTIONS: BTreeMap<String, Vec<String>> = BTreeMap::new();
static mut BG_JOBS: BTreeMap<u32, String> = BTreeMap::new();

fn main_loop() -> i32 {
    unsafe {
        ENV.insert("PATH".to_string(), "/bin:/usr/bin".to_string());
        ENV.insert("USER".to_string(), "root".to_string());
        ENV.insert("HOME".to_string(), "/home/root".to_string());
        ENV.insert("SHELL".to_string(), "/bin/sargash".to_string());
    }

    load_history();

    loop {
        unsafe { syscall3(SYS_WRITE, 1, PROMPT.as_ptr() as u64, PROMPT.len() as u64); }
        let line = read_line();
        if line.is_empty() { continue; }
        if line == "exit" { break; }

        add_history(line.clone());
        execute_line(line);
    }
    0
}

sarga_main!(main_loop);

fn read_line() -> String {
    let mut line = String::new();
    let mut buf = [0u8; 1];
    loop {
        let n = unsafe { syscall3(SYS_READ, 0, buf.as_mut_ptr() as u64, 1) };
        if n <= 0 { break; }
        if buf[0] == b'\n' { break; }
        if buf[0] == b'\r' { continue; }
        line.push(buf[0] as char);
    }
    line
}

fn execute_line(_line: String) {
    // Basic execution placeholder for compilation fix
}

fn load_history() {}
fn add_history(_s: String) {}

