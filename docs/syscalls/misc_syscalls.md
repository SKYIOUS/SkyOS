# Miscellaneous System Calls

These syscalls don't fit neatly into other categories.

## uname (syscall 63)

```c
int uname(struct utsname *buf);
```

Returns system identification information including OS name, hostname, kernel release, kernel
version, machine architecture, and domain name. All fields are null-terminated strings.

## sysinfo (syscall 203)

```c
int sysinfo(struct sysinfo *info);
```

Returns overall system statistics: total/available RAM, swap usage, process count, load averages,
and uptime.

## getcwd / chdir (syscalls 79-80)

```c
char *getcwd(char *buf, size_t size);
int chdir(const char *path);
```

Manages the current working directory. `getcwd` returns the absolute path of the CWD. `chdir`
changes the CWD to the specified path.

## access (syscall 21)

```c
int access(const char *pathname, int mode);
```

Checks whether the calling process can access the file. `mode` is a bitmask of `R_OK`, `W_OK`,
`X_OK`, and `F_OK`.

## sched_yield (syscall 24)

```c
int sched_yield(void);
```

Voluntarily yields the CPU to allow other tasks to run. Returns 0 on success.

## pause (syscall 34)

```c
int pause(void);
```

Suspends the calling process until a signal is received. Always returns -1 with `EINTR`.

## Signal syscalls (13/15/309)

```c
int rt_sigaction(int signum, const struct sigaction *act, struct sigaction *oldact);   // 13
int rt_sigreturn(void);   // 15
int sigprocmask(int how, const sigset_t *set, sigset_t *oldset);   // 309
```

Signal handling operations.

## futex (syscall 202)

```c
int futex(int *uaddr, int futex_op, int val, const struct timespec *timeout, int *uaddr2, int val3);
```

Fast userspace mutex operation. Used for implementing efficient userspace synchronization.

## set_tid_address (syscall 218)

```c
void *set_tid_address(int *tidptr);
```

Sets the pointer to the thread ID for thread-local storage.
