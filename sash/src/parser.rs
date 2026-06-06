#![no_std]
use alloc::string::String;
use alloc::vec::Vec;

pub fn parse(input: &str) -> Vec<String> {
    // Basic whitespace split
    input.split_whitespace().map(|s| String::from(s)).collect()
}
