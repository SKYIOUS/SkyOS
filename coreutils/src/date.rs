#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{sarga_main, println, args, syscall};

#[repr(C)]
struct TimeSpec {
    tv_sec: u64,
    tv_nsec: u64,
}

fn format_timestamp(sec: u64, _nsec: u64) -> alloc::string::String {
    // Basic formatting: YYYY-MM-DD HH:MM:SS (UTC)
    // Epoch is 1970-01-01
    let mut s = sec;
    let seconds = s % 60; s /= 60;
    let minutes = s % 60; s /= 60;
    let hours = s % 24; s /= 24;

    // Simplistic day to date conversion (ignoring leap years for brevity in this OS)
    let days = s;
    let year = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;

    alloc::format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", year, month, day, hours, minutes, seconds)
}

fn user_main() -> i32 {
    let mut saw_help = false;
    if args::argc() > 1 {
        for i in 1..args::argc() {
            if let Some(arg) = args::get(i as usize) {
                if arg == "--help" || arg == "-h" {
                    saw_help = true;
                }
            }
        }
    }

    if saw_help {
        libsarga::println!("Usage: date");
        libsarga::println!("Print the current SARGA OS date and time in UTC.");
        return 0;
    }

    let mut ts = TimeSpec { tv_sec: 0, tv_nsec: 0 };
    // clock_gettime: syscall2(228, CLOCK_REALTIME=0, ts_ptr)
    let r = unsafe { syscall::syscall2(228, 0, &mut ts as *mut TimeSpec as u64) };
    if r == 0 {
        libsarga::println!("{}", format_timestamp(ts.tv_sec, ts.tv_nsec));
    } else {
        libsarga::println!("date: unable to read clock");
        return 1;
    }
    0
    0
}
sarga_main!(user_main);
