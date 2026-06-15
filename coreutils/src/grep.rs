#![no_std]
#![no_main]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use libsarga::{sarga_main, println, print, args, io};

fn simple_match(pattern: &str, line: &str, case_insensitive: bool) -> bool {
    let (p, l) = if case_insensitive {
        (pattern.to_lowercase(), line.to_lowercase())
    } else {
        (String::from(pattern), String::from(line))
    };
    l.contains(&p)
}

fn user_main() {
    let mut _recursive = false;
    let mut case_insensitive = false;
    let mut invert = false;
    let mut line_num = false;
    let mut count_only = false;
    let mut files_with_matches = false;
    let mut pattern = String::new();
    let mut files = Vec::new();
    let mut i = 1;

    while i < args::argc() {
        if let Some(s) = args::get(i as usize) {
            if s == "-r" || s == "-R" { _recursive = true; i += 1; }
            else if s == "-i" { case_insensitive = true; i += 1; }
            else if s == "-v" { invert = true; i += 1; }
            else if s == "-n" { line_num = true; i += 1; }
            else if s == "-c" { count_only = true; i += 1; }
            else if s == "-l" { files_with_matches = true; i += 1; }
            else if s == "-rv" || s == "-vr" { _recursive = true; invert = true; i += 1; }
            else if s == "-ri" || s == "-ir" { _recursive = true; case_insensitive = true; i += 1; }
            else if s == "-in" || s == "-ni" { line_num = true; case_insensitive = true; i += 1; }
            else if s.starts_with('-') { i += 1; }
            else {
                if pattern.is_empty() { pattern = String::from(s); }
                else { files.push(String::from(s)); }
                i += 1;
            }
        } else { break; }
    }

    if pattern.is_empty() { println!("Usage: grep [-r] [-i] [-v] [-n] [-c] [-l] <pattern> [file...]"); return; }

    let read_fd = |fd: i64| -> Vec<String> {
        let mut lines = Vec::new();
        let mut buf = [0u8; 1024];
        let mut leftover = String::new();
        loop {
            let n = match io::read(fd, &mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
            leftover.push_str(s);
            while let Some(pos) = leftover.find('\n') {
                lines.push(String::from(&leftover[..pos]));
                leftover = String::from(&leftover[pos + 1..]);
            }
        }
        if !leftover.is_empty() { lines.push(leftover); }
        lines
    };

    let grep_lines = |lines: &[String], filename: &str, show_filename: bool| {
        let mut match_count = 0;
        let mut printed = false;
        for (idx, line) in lines.iter().enumerate() {
            let matched = simple_match(&pattern, line, case_insensitive);
            let show = if invert { !matched } else { matched };
            if show {
                match_count += 1;
                if count_only { continue; }
                if files_with_matches { if !printed { println!("{}", filename); printed = true; } continue; }
                if show_filename { print!("{}:", filename); }
                if line_num { print!("{}:", idx + 1); }
                println!("{}", line);
            }
        }
        if count_only {
            if show_filename { print!("{}:", filename); }
            println!("{}", match_count);
        }
    };

    if files.is_empty() {
        let mut buf = [0u8; 1024];
        let mut leftover = String::new();
        let mut line_idx = 0;
        loop {
            let n = match io::read(0, &mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => break,
            };
            let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
            leftover.push_str(s);
            while let Some(pos) = leftover.find('\n') {
                let line = &leftover[..pos];
                line_idx += 1;
                let matched = simple_match(&pattern, line, case_insensitive);
                let show = if invert { !matched } else { matched };
                if show {
                    if count_only { continue; }
                    if line_num { print!("{}:", line_idx); }
                    io::write_all(1, line.as_bytes()).ok();
                    io::write_all(1, b"\n").ok();
                }
                leftover = String::from(&leftover[pos + 1..]);
            }
        }
        if !leftover.is_empty() {
            line_idx += 1;
            let matched = simple_match(&pattern, &leftover, case_insensitive);
            let show = if invert { !matched } else { matched };
            if show {
                if line_num { print!("{}:", line_idx); }
                io::write_all(1, leftover.as_bytes()).ok();
                io::write_all(1, b"\n").ok();
            }
        }
    } else {
        let show_filename = files.len() > 1;
        for file in &files {
            let fd = match io::open(file, 0) {
                Ok(fd) => fd,
                Err(_) => { println!("grep: {}: not found", file); continue; }
            };
            let lines = read_fd(fd);
            let _ = io::close(fd);
            grep_lines(&lines, file, show_filename);
        }
    }
}

sarga_main!(user_main);
