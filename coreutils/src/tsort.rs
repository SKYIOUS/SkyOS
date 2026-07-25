#![no_std]
#![no_main]
extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use libsarga::{args, io, println, sarga_main};

fn user_main() -> i32 {
    let input = if args::argc() > 1 {
        match io::read_to_string(args::get(1).unwrap()) { Ok(s) => s, Err(_) => { println!("tsort: error"); return 1; } }
    } else {
        let mut buf = [0u8; 4096]; let mut all = String::new();
        loop { match io::read(0, &mut buf) { Ok(0) => break, Ok(n) => { if let Ok(s) = core::str::from_utf8(&buf[..n]) { all.push_str(s); } }, Err(_) => break, } }
        all
    };

    let mut deps: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in input.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 { deps.entry(parts[0].to_string()).or_default().push(parts[1].to_string()); }
    }

    // Simple topological sort (Kahn's algorithm)
    let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
    for (k, vs) in &deps {
        in_degree.entry(k.clone()).or_insert(0);
        for v in vs { *in_degree.entry(v.clone()).or_insert(0) += 1; }
    }

    let mut queue: Vec<String> = in_degree.iter().filter(|(_, &d)| d == 0).map(|(k, _)| k.clone()).collect();
    let mut sorted = Vec::new();
    while let Some(node) = queue.pop() {
        sorted.push(node.clone());
        if let Some(neighbors) = deps.get(&node) {
            for n in neighbors {
                if let Some(d) = in_degree.get_mut(n) {
                    *d -= 1;
                    if *d == 0 { queue.push(n.clone()); }
                }
            }
        }
    }

    for s in &sorted { println!("{}", s); }
    0
}
sarga_main!(user_main);