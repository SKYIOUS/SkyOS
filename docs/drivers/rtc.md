# Real-Time Clock Driver

The Real-Time Clock (RTC) driver (`kernel/kernel/src/drivers/rtc.rs`) provides battery-backed wall-clock time.

## Hardware Interface

The RTC is accessed through CMOS memory at I/O ports 0x70 (index) and 0x71 (data). The driver reads time fields as binary-coded decimal (BCD) values and converts them to binary.

## Public API

```rust
pub fn init() -> Result<(), ()>;          // read CMOS time, store epoch base
pub fn cleanup();                         // clear initialized flag
pub fn read_realtime() -> (i64, i64);     // (seconds, nanoseconds) since epoch
```

There is no `RtcTime` struct; time is returned as a (secs, nsecs) tuple.

## Time Reading

`cmos_read_time()`:
1. Waits for the update-in-progress flag (`STATUS_UPDATE_IN_PROGRESS` in status reg A) to clear
2. Reads second/minute/hour/day/month/year (registers 0x00–0x09), converting BCD→binary
3. Reads the second field again after the next update; retries until two consecutive reads match
4. Computes Unix seconds via `days_from_ymd` (Howard Hinnant's algorithm)

## Realtime

`init()` stores the CMOS epoch as `RTC_EPOCH_SECS`. `read_realtime()` returns `epoch_base + elapsed_ms/1000` seconds and the millisecond remainder as nanoseconds, where elapsed time is derived from the timer ticks (`interrupts::get_ticks() * 10ms`). Reads return `(0,0)` before `init()` succeeds.

Note: NMI is disabled on every CMOS read (0x80 mask), and time is always read in BCD regardless of register B's binary flag.
