#![no_std]
#![no_main]
extern crate alloc;

use libsarga::{args, sarga_main, syscall::*};

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096).div_euclid(365);
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2).div_euclid(153);
    let day = (doy - (153 * mp + 2).div_euclid(5) + 1) as u32;
    let month = (mp + if mp < 10 { 3 } else { -9 }) as u32;
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

fn format_timestamp(sec: i64, nsec: i64) -> alloc::string::String {
    let day_secs = sec.div_euclid(86_400);
    let secs_of_day = sec.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(day_secs);
    let hour = (secs_of_day / 3_600) as u32;
    let minute = ((secs_of_day % 3_600) / 60) as u32;
    let second = (secs_of_day % 60) as u32;

    if nsec > 0 {
        alloc::format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:09} UTC",
            year,
            month,
            day,
            hour,
            minute,
            second,
            nsec as u32
        )
    } else {
        alloc::format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
            year,
            month,
            day,
            hour,
            minute,
            second
        )
    }
}

fn user_main() {
    if args::argc() > 1 {
        let mut saw_help = false;
        for i in 1..args::argc() {
            if let Some(arg) = args::get(i as usize) {
                if arg == "--help" || arg == "-h" {
                    saw_help = true;
                }
            }
        }
        if saw_help {
            libsarga::println!("Usage: date");
            libsarga::println!("Print the current SkyOS date and time in UTC.");
            return;
        }
    }

    let mut ts = Timespec { tv_sec: 0, tv_nsec: 0 };
    let r = unsafe { syscall2(228, 0, (&mut ts as *mut Timespec) as u64) };
    if r == 0 {
        libsarga::println!("{}", format_timestamp(ts.tv_sec, ts.tv_nsec));
    } else {
        libsarga::println!("date: unable to read clock");
    }
}

sarga_main!(user_main);
