#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, io};
use alloc::string::String;
use alloc::vec::Vec;

fn print_usage() {
    println!("spkg - Sarga Package Manager");
    println!("Usage: spkg <command> [package]");
    println!("Commands:");
    println!("  install <pkg>   Install a package");
    println!("  remove <pkg>    Remove a package");
    println!("  update          Refresh package index");
    println!("  list            List installed packages");
}

fn user_main() {
    // We simulate argv parsing here by reading it from somewhere, 
    // but without real argv passed from crt0, we just print usage.
    print_usage();
}

sarga_main!(user_main);
