#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::{String, ToString};
use libsarga::{sarga_main, println, fs, args};

fn user_main() {
    let paths: Vec<String> = if args::argc() > 1 {
        (1..args::argc()).filter_map(|i| args::get(i as usize)).map(|s| s.to_string()).collect()
    } else {
        vec!["/".into(), "/tmp".into(), "/dev".into(), "/ctl".into()]
    };

    let human = true;
    println!("Filesystem      Size     Used    Avail   Use%  Mounted on");
    for path in &paths {
        let p: &str = path.as_str();
        let label: &str = match p {
            "/" => "/dev/initrd",
            "/tmp" => "tmpfs",
            "/dev" => "devfs",
            "/ctl" => "ctlfs",
            other => other,
        };
        match fs::statfs(path) {
            Ok(s) => {
                let total = s.f_blocks * s.f_bsize / 1024;
                let free = s.f_bfree * s.f_bsize / 1024;
                let avail = s.f_bavail * s.f_bsize / 1024;
                let used = total.saturating_sub(free);
                let pct = if total > 0 { used * 100 / total } else { 0 };
                if human {
                    let (ts, tu, ta) = (fmt_size(total * 1024), fmt_size(used * 1024), fmt_size(avail * 1024));
                    println!("{:<14} {:>6} {:>6} {:>6}  {:>3}%  {}", label, ts, tu, ta, pct, path);
                } else {
                    println!("{:<14} {:>6} {:>6} {:>6}  {:>3}%  {}", label, total, used, avail, pct, path);
                }
            }
            Err(_) => {
                if human {
                    println!("{:<14} {:>6} {:>6} {:>6}  {:>3}%  {}", label, "-", "-", "-", 0, path);
                } else {
                    println!("{:<14} {:>6} {:>6} {:>6}  {:>3}%  {}", label, 0, 0, 0, 0, path);
                }
            }
        }
    }
}

fn fmt_size(kb: u64) -> String {
    if kb >= 1024 * 1024 {
        alloc::format!("{:.1}G", kb as f64 / (1024.0 * 1024.0))
    } else if kb >= 1024 {
        alloc::format!("{:.1}M", kb as f64 / 1024.0)
    } else if kb >= 1 {
        alloc::format!("{}K", kb / 1024)
    } else {
        alloc::format!("{}B", kb)
    }
}

sarga_main!(user_main);
