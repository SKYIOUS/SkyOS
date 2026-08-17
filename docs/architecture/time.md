# Timekeeping and Timer Subsystem

SkyOS maintains monotonic and wall-clock time through a combination of hardware timers and software timekeeping.

## Time Sources

The kernel uses the following time sources:
- **LAPIC timer**: Drives the scheduler tick (10ms) and the monotonic counter (`interrupts::get_ticks()`)
- **RTC** (Real-Time Clock): Battery-backed wall-clock time (`drivers::rtc::read_realtime()`)
- **TSC** (Time Stamp Counter): Used for entropy/rdtsc, not as a primary clock

**There is no HPET driver and no PIT-based kernel clock.**

## Timer Management

**There is no timer wheel.** Timer management is limited to:
- LAPIC periodic tick for scheduling
- Per-thread `sleep_until` deadlines for `sys_nanosleep`
- The async executor (`task/executor.rs`) for event-driven work

## Clock IDs

`sys_clock_gettime` (`syscalls/mod.rs`) exposes exactly two clocks:
- `CLOCK_MONOTONIC` (1): `ticks * 10ms` from the LAPIC tick counter
- `CLOCK_REALTIME` (0): RTC wall-clock time

`CLOCK_THREAD_CPUTIME` and `CLOCK_PROCESS_CPUTIME` are **not implemented**.

## Sleep and Timeouts

The `nanosleep()` syscall (`sys_nanosleep`) suspends the calling task by setting a `sleep_until` deadline; the LAPIC tick wakes it. Timeouts on IPC and I/O are handled separately (non-blocking sockets, pre-check EINTR on accept/read).

## NTP Support

`adjtimex()` is **not implemented** — no clock slewing exists.
