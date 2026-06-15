#![no_std]
#![no_main]
extern crate alloc;
use libsarga::sarga_main;
use libsarga::io;
use libsarga::args;

fn read_all_stdin() -> alloc::vec::Vec<u8> {
    let mut data = alloc::vec::Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match io::read(0, &mut buf) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&buf[..n]),
            Err(_) => break,
        }
    }
    data
}

fn user_main() {
    let mut delim = b'\t';
    let mut fields: alloc::vec::Vec<(usize, usize)> = alloc::vec::Vec::new();
    let mut i = 1;
    while i < args::argc() {
        let arg = args::get(i as usize).unwrap_or("");
        if arg.starts_with("-d") {
            let d = if arg.len() > 2 { &arg[2..] } else {
                i += 1;
                args::get(i as usize).unwrap_or("")
            };
            delim = d.as_bytes()[0];
        } else if arg.starts_with("-f") {
            let f = if arg.len() > 2 { &arg[2..] } else {
                i += 1;
                args::get(i as usize).unwrap_or("")
            };
            for part in f.split(',') {
                if let Some((a, b)) = part.split_once('-') {
                    if let (Ok(s), Ok(e)) = (a.parse::<usize>(), b.parse::<usize>()) {
                        fields.push((s, e));
                    }
                } else if let Ok(n) = part.parse::<usize>() {
                    fields.push((n, n));
                }
            }
        }
        i += 1;
    }
    if fields.is_empty() {
        io::print_str("Usage: cut -d<delim> -f<fields>\n");
        return;
    }
    let data = read_all_stdin();
    let text = alloc::string::String::from_utf8_lossy(&data);
    for line in text.lines() {
        let parts: alloc::vec::Vec<&str> = line.split(|c| c == delim as char).collect();
        let mut first = true;
        for &(start, end) in &fields {
            if start >= 1 && start <= parts.len() {
                let s = start - 1;
                let e = core::cmp::min(end, parts.len());
                if !first {
                    let mut db = [0u8; 1];
                    db[0] = delim;
                    let _ = io::write(1, &db);
                }
                first = false;
                for p in &parts[s..e] {
                    let _ = io::write(1, p.as_bytes());
                }
            }
        }
        let _ = io::write(1, b"\n");
    }
}

sarga_main!(user_main);
