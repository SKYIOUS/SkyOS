# Time System Calls

The time syscalls provide timing and sleep functionality.

## clock_gettime (syscall 228)

```c
int clock_gettime(clockid_t clockid, struct timespec *tp);
```

Retrieves the time from the specified clock.

**Clock IDs** (only these two are implemented):
| ID | Description |
|----|-------------|
| 0 (CLOCK_REALTIME) | Wall-clock time |
| 1 (CLOCK_MONOTONIC) | Time since boot |

## nanosleep (syscall 35)

```c
int nanosleep(const struct timespec *req, struct timespec *rem);
```

Suspends the calling thread until the specified time has elapsed. The `rem` parameter returns the
remaining time if the sleep was interrupted (e.g. by a signal).

## POSIX Timer Syscalls (222-226)

```c
int timer_create(clockid_t clockid, struct sigevent *sevp, timer_t *timerid);   // 222
int timer_settime(timer_t timerid, int flags, const struct itimerspec *new_value, struct itimerspec *old_value);   // 223
int timer_gettime(timer_t timerid, struct itimerspec *curr_value);   // 224
int timer_getoverrun(timer_t timerid);   // 225
int timer_delete(timer_t timerid);   // 226
```

POSIX timer management. Timers can deliver signals or notification on expiration.

## getitimer / setitimer (syscalls 350-351)

```c
int getitimer(int which, struct itimerval *curr_value);
int setitimer(int which, const struct itimerval *new_value, struct itimerval *old_value);
```

Interval timer management with microsecond resolution.
