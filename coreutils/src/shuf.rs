#![no_std]
#![no_main]
extern crate alloc;
use alloc::vec::Vec;
use core::num::Wrapping;
use libsarga::{args, io, println, sarga_main};

struct XorShift { state: Wrapping<u64> }
impl XorShift {
    fn new(seed: u64) -> Self { XorShift { state: Wrapping(seed) } }
    fn next(&mut self) -> u64 {
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.0.wrapping_mul(0x2545F4914F6CDD1Du64)
    }
    fn range(&mut self, max: usize) -> usize {
        if max == 0 { return 0; }
        (self.next() as usize) % max
    }
}

fn user_main() -> i32 {
    let lines: Vec<alloc::string::String> = if args::argc() > 1 {
        match io::read_to_string(args::get(1).unwrap()) {
            Ok(s) => s.lines().map(|l| l.to_string()).collect(),
            Err(_) => { println!("shuf: error"); return 1; }
        }
    } else {
        let mut buf = [0u8; 4096]; let mut all = alloc::string::String::new();
        loop { match io::read(0, &mut buf) { Ok(0) => break, Ok(n) => { if let Ok(s) = core::str::from_utf8(&buf[..n]) { all.push_str(s); } }, Err(_) => break, } }
        all.lines().map(|l| l.to_string()).collect()
    };

    let mut rng = XorShift::new(42);
    let mut shuffled = lines.clone();
    let n = shuffled.len();
    for i in (1..n).rev() {
        let j = rng.range(i + 1);
        shuffled.swap(i, j);
    }
    for line in &shuffled { println!("{}", line); }
    0
}
sarga_main!(user_main);