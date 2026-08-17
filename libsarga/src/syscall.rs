//! Raw x86_64 SYSCALL wrappers.
//! These are the ONLY place in Sarga OS that uses inline asm for syscalls.

/// # Safety
/// Raw syscall: caller must uphold the kernel ABI (all register arguments are
/// passed through unchecked).
#[inline(always)]
pub unsafe fn syscall6(n: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64) -> i64 {
    let ret: i64;
    core::arch::asm!(
        "syscall",
        inout("rax") n => ret,
        in("rdi") a1, in("rsi") a2, in("rdx") a3,
        in("r10") a4, in("r8") a5, in("r9") a6,
        out("rcx") _, out("r11") _,
        options(nostack)
    );
    ret
}

/// # Safety
/// Raw syscall: caller must uphold the kernel ABI (all register arguments are
/// passed through unchecked).
#[inline(always)]
pub unsafe fn syscall0(n: u64) -> i64 {
    syscall6(n, 0, 0, 0, 0, 0, 0)
}
/// # Safety
/// Raw syscall: caller must uphold the kernel ABI (all register arguments are
/// passed through unchecked).
#[inline(always)]
pub unsafe fn syscall1(n: u64, a1: u64) -> i64 {
    syscall6(n, a1, 0, 0, 0, 0, 0)
}
/// # Safety
/// Raw syscall: caller must uphold the kernel ABI (all register arguments are
/// passed through unchecked).
#[inline(always)]
pub unsafe fn syscall2(n: u64, a1: u64, a2: u64) -> i64 {
    syscall6(n, a1, a2, 0, 0, 0, 0)
}
/// # Safety
/// Raw syscall: caller must uphold the kernel ABI (all register arguments are
/// passed through unchecked).
#[inline(always)]
pub unsafe fn syscall3(n: u64, a1: u64, a2: u64, a3: u64) -> i64 {
    syscall6(n, a1, a2, a3, 0, 0, 0)
}
/// # Safety
/// Raw syscall: caller must uphold the kernel ABI (all register arguments are
/// passed through unchecked).
#[inline(always)]
pub unsafe fn syscall4(n: u64, a1: u64, a2: u64, a3: u64, a4: u64) -> i64 {
    syscall6(n, a1, a2, a3, a4, 0, 0)
}
/// # Safety
/// Raw syscall: caller must uphold the kernel ABI (all register arguments are
/// passed through unchecked).
#[inline(always)]
pub unsafe fn syscall5(n: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> i64 {
    syscall6(n, a1, a2, a3, a4, a5, 0)
}

pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_OPEN: u64 = 2;
pub const SYS_CLOSE: u64 = 3;
pub const SYS_STAT: u64 = 4;
pub const SYS_FSTAT: u64 = 5;
pub const SYS_LSEEK: u64 = 8;
pub const SYS_MMAP: u64 = 9;
pub const SYS_MUNMAP: u64 = 11;
pub const SYS_BRK: u64 = 12;
pub const SYS_RT_SIGACTION: u64 = 13;
pub const SYS_RT_SIGRETURN: u64 = 15;
pub const SYS_IOCTL: u64 = 16;
pub const SYS_PIPE: u64 = 22;
pub const SYS_DUP2: u64 = 33;
pub const SYS_NANOSLEEP: u64 = 35;
pub const SYS_GETPID: u64 = 39;
pub const SYS_SOCKET: u64 = 41;
pub const SYS_CONNECT: u64 = 42;
pub const SYS_SENDTO: u64 = 44;
pub const SYS_RECVFROM: u64 = 45;
pub const SYS_BIND: u64 = 49;
pub const SYS_CLONE: u64 = 56;
pub const SYS_FORK: u64 = 57;
pub const SYS_EXECVE: u64 = 59;
pub const SYS_EXIT: u64 = 60;
pub const SYS_WAIT4: u64 = 61;
pub const SYS_KILL: u64 = 62;
pub const SYS_UNAME: u64 = 63;
pub const SYS_GETCWD: u64 = 79;
pub const SYS_CHDIR: u64 = 80;
pub const SYS_MKDIR: u64 = 83;
pub const SYS_UNLINK: u64 = 87;
pub const SYS_SYMLINK: u64 = 88;
pub const SYS_READLINK: u64 = 89;
pub const SYS_RENAME: u64 = 82;
pub const SYS_FCHMOD: u64 = 91;
pub const SYS_FCHOWN: u64 = 93;
pub const SYS_GETDENTS64: u64 = 217;
pub const SYS_SCHED_YIELD: u64 = 24;
pub const SYS_CLOCK_GETTIME: u64 = 228;
pub const SYS_FUTEX: u64 = 202;
pub const SYS_MOUNT: u64 = 165;
pub const SYS_UMOUNT2: u64 = 167;
pub const SYS_ARCH_PRCTL: u64 = 158;
pub const SYS_RESOLVE: u64 = 200;
pub const SYS_BEEP: u64 = 104;
pub const SYS_SELECT: u64 = 23;
pub const SYS_POLL: u64 = 7;
pub const SYS_GETTID: u64 = 186;
pub const SYS_GETUID: u64 = 301;
pub const SYS_GETGID: u64 = 302;
pub const SYS_SETUID: u64 = 303;
pub const SYS_SETGID: u64 = 304;
pub const SYS_GETEUID: u64 = 305;
pub const SYS_GETEGID: u64 = 306;
pub const SYS_GETRESUID: u64 = 118;
pub const SYS_SETRESUID: u64 = 119;
pub const SYS_GETRESGID: u64 = 314;
pub const SYS_SETRESGID: u64 = 315;
pub const SYS_GETGROUPS: u64 = 115;
pub const SYS_SETGROUPS: u64 = 116;

pub const SYS_GETPPID: u64 = 110;
pub const SYS_GETPGRP: u64 = 111;
pub const SYS_SETSID: u64 = 112;
pub const SYS_GETPGID: u64 = 330;
pub const SYS_SETPGID: u64 = 157;
pub const SYS_GETSID: u64 = 331;
pub const SYS_GETRLIMIT: u64 = 97;
pub const SYS_SETRLIMIT: u64 = 98;
pub const SYS_PRLIMIT64: u64 = 332;

pub const SYS_SIGALTSTACK: u64 = 131;
pub const SYS_SIGNALFD: u64 = 282;
pub const SYS_SIGNALFD4: u64 = 289;
pub const SYS_PAUSE: u64 = 34;
pub const SYS_GETITIMER: u64 = 350;
pub const SYS_SETITIMER: u64 = 351;
pub const SYS_TIMES: u64 = 352;

pub const SYS_SHMGET: u64 = 29;
pub const SYS_SHMAT: u64 = 30;
pub const SYS_SHMCTL: u64 = 31;
pub const SYS_SHMDT: u64 = 67;
pub const SYS_MEMFD_CREATE: u64 = 319;

pub const SYS_TIMER_CREATE: u64 = 222;
pub const SYS_TIMER_SETTIME: u64 = 223;
pub const SYS_TIMER_GETTIME: u64 = 224;
pub const SYS_TIMER_GETOVERRUN: u64 = 225;
pub const SYS_TIMER_DELETE: u64 = 226;

pub const SYS_LINK: u64 = 86;
pub const SYS_LSTAT: u64 = 6;
pub const SYS_UTIMENSAT: u64 = 280;
pub const SYS_FALLOCATE: u64 = 285;
pub const SYS_SENDFILE: u64 = 40;

pub const SYS_EVENTFD: u64 = 284;
pub const SYS_EVENTFD2: u64 = 290;

pub const SYS_OPENAT: u64 = 257;
pub const SYS_MKDIRAT: u64 = 258;
pub const SYS_FSTATAT: u64 = 262;
pub const SYS_UNLINKAT: u64 = 263;
pub const SYS_RENAMEAT: u64 = 264;
pub const SYS_LINKAT: u64 = 265;
pub const SYS_SYMLINKAT: u64 = 266;
pub const SYS_READLINKAT: u64 = 267;
pub const SYS_FACCESSAT: u64 = 269;

pub const SYS_SOCKETPAIR: u64 = 53;
pub const SYS_SETSOCKOPT: u64 = 54;
pub const SYS_GETSOCKOPT: u64 = 55;
pub const SYS_SENDMSG: u64 = 46;
pub const SYS_RECVMSG: u64 = 47;
pub const SYS_GETSOCKNAME: u64 = 51;
pub const SYS_GETPEERNAME: u64 = 52;

pub const SYS_CHMOD: u64 = 90;
pub const SYS_UMASK: u64 = 95;
pub const SYS_SYNC: u64 = 36;
pub const SYS_STATFS: u64 = 137;
pub const SYS_MPROTECT: u64 = 10;
pub const SYS_FCNTL: u64 = 72;
pub const SYS_SCHED_SETATTR: u64 = 144;
pub const SYS_SCHED_GETATTR: u64 = 145;
pub const SYS_CAPGET: u64 = 307;
pub const SYS_CAPSET: u64 = 308;
pub const SYS_SIGPROCMASK: u64 = 309;
pub const SYS_SYSINFO: u64 = 203;
pub const SYS_SET_TID_ADDRESS: u64 = 218;
pub const SYS_EXIT_GROUP: u64 = 231;
pub const SYS_TRUNCATE: u64 = 76;
pub const SYS_FTRUNCATE: u64 = 77;
pub const SYS_SWAPON: u64 = 326;
pub const SYS_SWAPOFF: u64 = 327;

/// # Safety
/// Caller must ensure `buf` points to valid writable memory of at least `len` bytes.
pub unsafe fn read(fd: i64, buf: *mut u8, len: usize) -> i64 {
    syscall3(SYS_READ, fd as u64, buf as u64, len as u64)
}
/// # Safety
/// Caller must ensure `buf` points to valid readable memory of at least `len` bytes.
pub unsafe fn write(fd: i64, buf: *const u8, len: usize) -> i64 {
    syscall3(SYS_WRITE, fd as u64, buf as u64, len as u64)
}
/// # Safety
/// Caller must ensure `path` points to a valid NUL-terminated string.
pub unsafe fn open(path: *const u8, flags: i32) -> i64 {
    syscall2(SYS_OPEN, path as u64, flags as u64)
}
/// # Safety
/// Caller must ensure `fd` is a valid open descriptor; no pointer arguments.
pub unsafe fn close(fd: i64) -> i64 {
    syscall1(SYS_CLOSE, fd as u64)
}
/// # Safety
/// Caller must uphold the kernel fork ABI; no pointer arguments.
pub unsafe fn fork() -> i64 {
    syscall0(SYS_FORK)
}
/// # Safety
/// Caller must ensure `path`/`argv`/`envp` point to valid, NUL-terminated data
/// for the duration of the call.
pub unsafe fn execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> i64 {
    syscall3(SYS_EXECVE, path as u64, argv as u64, envp as u64)
}
/// # Safety
/// Caller must uphold the exit ABI; this function never returns.
pub unsafe fn exit(code: i32) -> ! {
    syscall1(SYS_EXIT, code as u64);
    loop {
        core::hint::spin_loop();
    }
}
/// # Safety
/// Caller must ensure `status`/`rusage` point to valid writable memory.
pub unsafe fn wait4(pid: i64, status: *mut i32, options: i32, rusage: *mut u8) -> i64 {
    syscall4(
        SYS_WAIT4,
        pid as u64,
        status as u64,
        options as u64,
        rusage as u64,
    )
}
/// # Safety
/// Caller must ensure `buf` points to valid writable memory of at least `len` bytes.
pub unsafe fn getdents64(fd: i64, buf: *mut u8, len: usize) -> i64 {
    syscall3(SYS_GETDENTS64, fd as u64, buf as u64, len as u64)
}
/// # Safety
/// Caller must ensure `path` points to a valid NUL-terminated string.
pub unsafe fn unlink(path: *const u8) -> i64 {
    syscall1(SYS_UNLINK, path as u64)
}
/// # Safety
/// Caller must ensure `path` points to a valid NUL-terminated string.
pub unsafe fn mkdir(path: *const u8, mode: u32) -> i64 {
    syscall2(SYS_MKDIR, path as u64, mode as u64)
}
/// # Safety
/// Caller must ensure `buf` points to valid writable memory for the stat result.
pub unsafe fn fstat(fd: i64, buf: *mut u8) -> i64 {
    syscall2(SYS_FSTAT, fd as u64, buf as u64)
}

/// # Safety
/// Caller must uphold the beep ABI; no pointer arguments.
pub unsafe fn beep(freq: u32, duration: u32) -> i64 {
    syscall2(SYS_BEEP, freq as u64, duration as u64)
}

// Futex operations
pub const FUTEX_WAIT: u32 = 0;
pub const FUTEX_WAKE: u32 = 1;

/// Futex syscall wrapper.
/// Matches the kernel's SYS_FUTEX (202) ABI:
///   uaddr, op, val, timeout, uaddr2, val3
///
/// # Safety
/// Caller must ensure `uaddr` (and `uaddr2` when used) points to valid atomic
/// memory for the duration of the call.
pub unsafe fn sys_futex(
    uaddr: usize,
    op: u32,
    val: usize,
    timeout: usize,
    uaddr2: usize,
    val3: usize,
) -> i64 {
    syscall6(
        SYS_FUTEX,
        uaddr as u64,
        op as u64,
        val as u64,
        timeout as u64,
        uaddr2 as u64,
        val3 as u64,
    )
}

/// Get current thread ID.
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn sys_gettid() -> i64 {
    syscall0(SYS_GETTID)
}

// ── Process groups ────────────────────────────────────────────────
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn setpgid(pid: i64, pgid: i64) -> i64 {
    syscall2(SYS_SETPGID, pid as u64, pgid as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn getpgid(pid: i64) -> i64 {
    syscall1(SYS_GETPGID, pid as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn getpgrp() -> i64 {
    syscall0(SYS_GETPGRP)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn setsid() -> i64 {
    syscall0(SYS_SETSID)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn getsid(pid: i64) -> i64 {
    syscall1(SYS_GETSID, pid as u64)
}

// ── Resource limits ───────────────────────────────────────────────
/// # Safety
/// Caller must ensure `rlim` points to valid writable memory for the limit struct.
pub unsafe fn getrlimit(resource: i32, rlim: *mut u8) -> i64 {
    syscall2(SYS_GETRLIMIT, resource as u64, rlim as u64)
}
/// # Safety
/// Caller must ensure `rlim` points to a valid limit struct for the duration of the call.
pub unsafe fn setrlimit(resource: i32, rlim: *const u8) -> i64 {
    syscall2(SYS_SETRLIMIT, resource as u64, rlim as u64)
}
/// # Safety
/// Caller must ensure `new`/`old` point to valid limit structs for the duration of the call.
pub unsafe fn prlimit64(pid: i64, resource: i32, new: *const u8, old: *mut u8) -> i64 {
    syscall4(
        SYS_PRLIMIT64,
        pid as u64,
        resource as u64,
        new as u64,
        old as u64,
    )
}

// ── *at variants ──────────────────────────────────────────────────
/// # Safety
/// Caller must ensure `path` points to a valid NUL-terminated string.
pub unsafe fn openat(dirfd: i64, path: *const u8, flags: i32, mode: u32) -> i64 {
    syscall4(
        SYS_OPENAT,
        dirfd as u64,
        path as u64,
        flags as u64,
        mode as u64,
    )
}
/// # Safety
/// Caller must ensure `path` points to a valid NUL-terminated string.
pub unsafe fn mkdirat(dirfd: i64, path: *const u8, mode: u32) -> i64 {
    syscall3(SYS_MKDIRAT, dirfd as u64, path as u64, mode as u64)
}
/// # Safety
/// Caller must ensure `path` points to a valid NUL-terminated string.
pub unsafe fn unlinkat(dirfd: i64, path: *const u8, flags: i32) -> i64 {
    syscall3(SYS_UNLINKAT, dirfd as u64, path as u64, flags as u64)
}
/// # Safety
/// Caller must ensure `target`/`path` point to valid NUL-terminated strings.
pub unsafe fn symlinkat(target: *const u8, dirfd: i64, path: *const u8) -> i64 {
    syscall3(SYS_SYMLINKAT, target as u64, dirfd as u64, path as u64)
}
/// # Safety
/// Caller must ensure `path`/`buf` point to valid memory of at least `size` bytes.
pub unsafe fn readlinkat(dirfd: i64, path: *const u8, buf: *mut u8, size: usize) -> i64 {
    syscall4(
        SYS_READLINKAT,
        dirfd as u64,
        path as u64,
        buf as u64,
        size as u64,
    )
}
/// # Safety
/// Caller must ensure `oldpath`/`newpath` point to valid NUL-terminated strings.
pub unsafe fn renameat(
    olddirfd: i64,
    oldpath: *const u8,
    newdirfd: i64,
    newpath: *const u8,
) -> i64 {
    syscall4(
        SYS_RENAMEAT,
        olddirfd as u64,
        oldpath as u64,
        newdirfd as u64,
        newpath as u64,
    )
}
/// # Safety
/// Caller must ensure `path`/`buf` point to valid memory for the duration of the call.
pub unsafe fn fstatat(dirfd: i64, path: *const u8, buf: *mut u8, flags: i32) -> i64 {
    syscall4(
        SYS_FSTATAT,
        dirfd as u64,
        path as u64,
        buf as u64,
        flags as u64,
    )
}
/// # Safety
/// Caller must ensure `path` points to a valid NUL-terminated string.
pub unsafe fn faccessat(dirfd: i64, path: *const u8, mode: i32, flags: i32) -> i64 {
    syscall4(
        SYS_FACCESSAT,
        dirfd as u64,
        path as u64,
        mode as u64,
        flags as u64,
    )
}
/// # Safety
/// Caller must ensure `oldpath`/`newpath` point to valid NUL-terminated strings.
pub unsafe fn linkat(
    olddirfd: i64,
    oldpath: *const u8,
    newdirfd: i64,
    newpath: *const u8,
    flags: i32,
) -> i64 {
    syscall5(
        SYS_LINKAT,
        olddirfd as u64,
        oldpath as u64,
        newdirfd as u64,
        newpath as u64,
        flags as u64,
    )
}

// ── Socket/MSG ────────────────────────────────────────────────────
/// # Safety
/// Caller must ensure `msg` points to a valid msghdr struct for the duration of the call.
pub unsafe fn sendmsg(sockfd: i64, msg: *const u8, flags: i32) -> i64 {
    syscall3(SYS_SENDMSG, sockfd as u64, msg as u64, flags as u64)
}
/// # Safety
/// Caller must ensure `msg` points to a valid msghdr struct for the duration of the call.
pub unsafe fn recvmsg(sockfd: i64, msg: *mut u8, flags: i32) -> i64 {
    syscall3(SYS_RECVMSG, sockfd as u64, msg as u64, flags as u64)
}
/// # Safety
/// Caller must ensure `addr`/`addrlen` point to valid writable memory.
pub unsafe fn getsockname(sockfd: i64, addr: *mut u8, addrlen: *mut u32) -> i64 {
    syscall3(SYS_GETSOCKNAME, sockfd as u64, addr as u64, addrlen as u64)
}
/// # Safety
/// Caller must ensure `addr`/`addrlen` point to valid writable memory.
pub unsafe fn getpeername(sockfd: i64, addr: *mut u8, addrlen: *mut u32) -> i64 {
    syscall3(SYS_GETPEERNAME, sockfd as u64, addr as u64, addrlen as u64)
}
/// # Safety
/// Caller must ensure `optval`/`optlen` point to valid writable memory.
pub unsafe fn getsockopt(
    sockfd: i64,
    level: i32,
    optname: i32,
    optval: *mut u8,
    optlen: *mut u32,
) -> i64 {
    syscall5(
        SYS_GETSOCKOPT,
        sockfd as u64,
        level as u64,
        optname as u64,
        optval as u64,
        optlen as u64,
    )
}
/// # Safety
/// Caller must ensure `sv` points to writable memory for the two new fds.
pub unsafe fn socketpair(domain: u64, type_: u64, protocol: u64, sv: *mut i32) -> i64 {
    syscall4(SYS_SOCKETPAIR, domain, type_, protocol, sv as u64)
}

// ── Signals ───────────────────────────────────────────────────────
/// # Safety
/// Caller must ensure `ss`/`old_ss` point to valid StackT memory for the duration of the call.
pub unsafe fn sigaltstack(ss: *const u8, old_ss: *mut u8) -> i64 {
    syscall2(SYS_SIGALTSTACK, ss as u64, old_ss as u64)
}
/// # Safety
/// Caller must ensure `mask` points to valid readable memory of at least `size` bytes.
pub unsafe fn signalfd(fd: i64, mask: *const u64, size: usize) -> i64 {
    syscall3(SYS_SIGNALFD, fd as u64, mask as u64, size as u64)
}
/// # Safety
/// Caller must ensure `mask` points to valid readable memory of at least `size` bytes.
pub unsafe fn signalfd4(fd: i64, mask: *const u64, size: usize, flags: i32) -> i64 {
    syscall4(
        SYS_SIGNALFD4,
        fd as u64,
        mask as u64,
        size as u64,
        flags as u64,
    )
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn pause() -> i64 {
    syscall0(SYS_PAUSE)
}
/// # Safety
/// Caller must ensure `val` points to valid writable memory for the itimer result.
pub unsafe fn getitimer(which: i32, val: *mut u8) -> i64 {
    syscall2(SYS_GETITIMER, which as u64, val as u64)
}
/// # Safety
/// Caller must ensure `new`/`old` point to valid itimer memory for the duration of the call.
pub unsafe fn setitimer(which: i32, new: *const u8, old: *mut u8) -> i64 {
    syscall3(SYS_SETITIMER, which as u64, new as u64, old as u64)
}
/// # Safety
/// Caller must ensure `buf` points to valid writable memory for the times result.
pub unsafe fn times(buf: *mut u8) -> i64 {
    syscall1(SYS_TIMES, buf as u64)
}

// ── Shared memory ─────────────────────────────────────────────────
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn shmget(key: i32, size: usize, flags: i32) -> i64 {
    syscall3(SYS_SHMGET, key as u64, size as u64, flags as u64)
}
/// # Safety
/// Caller must ensure `addr` is a valid attach address or null.
pub unsafe fn shmat(shmid: i32, addr: *const u8, flags: i32) -> i64 {
    syscall3(SYS_SHMAT, shmid as u64, addr as u64, flags as u64)
}
/// # Safety
/// Caller must ensure `addr` points to a previously attached shm region.
pub unsafe fn shmdt(addr: *const u8) -> i64 {
    syscall1(SYS_SHMDT, addr as u64)
}
/// # Safety
/// Caller must ensure `buf` points to valid writable memory for the shm_info result.
pub unsafe fn shmctl(shmid: i32, cmd: i32, buf: *mut u8) -> i64 {
    syscall3(SYS_SHMCTL, shmid as u64, cmd as u64, buf as u64)
}
/// # Safety
/// Caller must ensure `name` points to a valid NUL-terminated string.
pub unsafe fn memfd_create(name: *const u8, flags: u32) -> i64 {
    syscall2(SYS_MEMFD_CREATE, name as u64, flags as u64)
}

// ── Timers ────────────────────────────────────────────────────────
/// # Safety
/// Caller must ensure `evp`/`timerid` point to valid memory for the duration of the call.
pub unsafe fn timer_create(clockid: i32, evp: *const u8, timerid: *mut i32) -> i64 {
    syscall3(SYS_TIMER_CREATE, clockid as u64, evp as u64, timerid as u64)
}
/// # Safety
/// Caller must ensure `new`/`old` point to valid itimerspec memory for the duration of the call.
pub unsafe fn timer_settime(timerid: i32, flags: i32, new: *const u8, old: *mut u8) -> i64 {
    syscall4(
        SYS_TIMER_SETTIME,
        timerid as u64,
        flags as u64,
        new as u64,
        old as u64,
    )
}
/// # Safety
/// Caller must ensure `val` points to valid writable memory for the itimerspec result.
pub unsafe fn timer_gettime(timerid: i32, val: *mut u8) -> i64 {
    syscall2(SYS_TIMER_GETTIME, timerid as u64, val as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn timer_getoverrun(timerid: i32) -> i64 {
    syscall1(SYS_TIMER_GETOVERRUN, timerid as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn timer_delete(timerid: i32) -> i64 {
    syscall1(SYS_TIMER_DELETE, timerid as u64)
}

// ── Credentials ───────────────────────────────────────────────────
/// # Safety
/// Caller must ensure `ruid`/`euid`/`suid` point to valid writable memory.
pub unsafe fn getresuid(ruid: *mut u32, euid: *mut u32, suid: *mut u32) -> i64 {
    syscall3(SYS_GETRESUID, ruid as u64, euid as u64, suid as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn setresuid(ruid: u32, euid: u32, suid: u32) -> i64 {
    syscall3(SYS_SETRESUID, ruid as u64, euid as u64, suid as u64)
}
/// # Safety
/// Caller must ensure `rgid`/`egid`/`sgid` point to valid writable memory.
pub unsafe fn getresgid(rgid: *mut u32, egid: *mut u32, sgid: *mut u32) -> i64 {
    syscall3(SYS_GETRESGID, rgid as u64, egid as u64, sgid as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn setresgid(rgid: u32, egid: u32, sgid: u32) -> i64 {
    syscall3(SYS_SETRESGID, rgid as u64, egid as u64, sgid as u64)
}
/// # Safety
/// Caller must ensure `list` points to valid writable memory of at least `size` entries.
pub unsafe fn getgroups(size: i32, list: *mut u32) -> i64 {
    syscall2(SYS_GETGROUPS, size as u64, list as u64)
}
/// # Safety
/// Caller must ensure `list` points to valid readable memory of at least `size` entries.
pub unsafe fn setgroups(size: i32, list: *const u32) -> i64 {
    syscall2(SYS_SETGROUPS, size as u64, list as u64)
}

// ── FS completion ─────────────────────────────────────────────────
/// # Safety
/// Caller must ensure `old`/`new` point to valid NUL-terminated strings.
pub unsafe fn link(old: *const u8, new: *const u8) -> i64 {
    syscall2(SYS_LINK, old as u64, new as u64)
}
/// # Safety
/// Caller must ensure `path`/`buf` point to valid memory for the duration of the call.
pub unsafe fn lstat(path: *const u8, buf: *mut u8) -> i64 {
    syscall2(SYS_LSTAT, path as u64, buf as u64)
}
/// # Safety
/// Caller must ensure `path`/`times` point to valid memory for the duration of the call.
pub unsafe fn utimensat(dirfd: i64, path: *const u8, times: *const u8, flags: i32) -> i64 {
    syscall4(
        SYS_UTIMENSAT,
        dirfd as u64,
        path as u64,
        times as u64,
        flags as u64,
    )
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn fallocate(fd: i64, mode: i32, offset: i64, len: i64) -> i64 {
    syscall4(
        SYS_FALLOCATE,
        fd as u64,
        mode as u64,
        offset as u64,
        len as u64,
    )
}
/// # Safety
/// Caller must ensure `offset` points to valid memory for the duration of the call.
pub unsafe fn sendfile(out_fd: i64, in_fd: i64, offset: *mut i64, count: usize) -> i64 {
    syscall4(
        SYS_SENDFILE,
        out_fd as u64,
        in_fd as u64,
        offset as u64,
        count as u64,
    )
}

// ── Event ─────────────────────────────────────────────────────────
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn eventfd(initval: u32, flags: i32) -> i64 {
    syscall2(SYS_EVENTFD, initval as u64, flags as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn eventfd2(initval: u32, flags: i32) -> i64 {
    syscall2(SYS_EVENTFD2, initval as u64, flags as u64)
}

// ── Misc ──────────────────────────────────────────────────────────
/// # Safety
/// Caller must ensure `path` points to a valid NUL-terminated string.
pub unsafe fn chmod(path: *const u8, mode: u32) -> i64 {
    syscall2(SYS_CHMOD, path as u64, mode as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn umask(mask: u32) -> i64 {
    syscall1(SYS_UMASK, mask as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn sys_sync() -> i64 {
    syscall0(SYS_SYNC)
}
/// # Safety
/// Caller must ensure `path`/`buf` point to valid memory for the duration of the call.
pub unsafe fn statfs(path: *const u8, buf: *mut u8) -> i64 {
    syscall2(SYS_STATFS, path as u64, buf as u64)
}
/// # Safety
/// Caller must ensure `addr`/`len` describe a valid mapped range.
pub unsafe fn mprotect(addr: u64, len: usize, prot: i32) -> i64 {
    syscall3(SYS_MPROTECT, addr, len as u64, prot as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn fcntl(fd: i64, cmd: i32, arg: u64) -> i64 {
    syscall3(SYS_FCNTL, fd as u64, cmd as u64, arg)
}
/// # Safety
/// Caller must ensure `attr` points to a valid sched_attr struct for the duration of the call.
pub unsafe fn sched_setattr(pid: i64, attr: *const u8, flags: u32) -> i64 {
    syscall3(SYS_SCHED_SETATTR, pid as u64, attr as u64, flags as u64)
}
/// # Safety
/// Caller must ensure `attr` points to valid writable memory of at least `size` bytes.
pub unsafe fn sched_getattr(pid: i64, attr: *mut u8, size: u32, flags: u32) -> i64 {
    syscall4(
        SYS_SCHED_GETATTR,
        pid as u64,
        attr as u64,
        size as u64,
        flags as u64,
    )
}
/// # Safety
/// Caller must ensure `hdr`/`data` point to valid writable memory.
pub unsafe fn capget(hdr: *mut u8, data: *mut u8) -> i64 {
    syscall2(SYS_CAPGET, hdr as u64, data as u64)
}
/// # Safety
/// Caller must ensure `hdr`/`data` point to valid readable memory for the duration of the call.
pub unsafe fn capset(hdr: *const u8, data: *const u8) -> i64 {
    syscall2(SYS_CAPSET, hdr as u64, data as u64)
}
/// # Safety
/// Caller must ensure `set`/`oldset` point to valid sigset memory for the duration of the call.
pub unsafe fn sys_sigprocmask(how: i32, set: *const u64, oldset: *mut u64) -> i64 {
    syscall3(SYS_SIGPROCMASK, how as u64, set as u64, oldset as u64)
}
/// # Safety
/// Caller must ensure `info` points to valid writable memory for the sysinfo result.
pub unsafe fn sys_sysinfo(info: *mut u8) -> i64 {
    syscall1(SYS_SYSINFO, info as u64)
}
/// # Safety
/// Caller must ensure `tidptr` points to valid writable memory for the duration of the call.
pub unsafe fn set_tid_address(tidptr: *const u32) -> i64 {
    syscall1(SYS_SET_TID_ADDRESS, tidptr as u64)
}
/// # Safety
/// Caller must uphold the exit ABI; this function never returns.
pub unsafe fn exit_group(code: i32) -> ! {
    syscall1(SYS_EXIT_GROUP, code as u64);
    loop {
        core::hint::spin_loop();
    }
}
/// # Safety
/// Caller must ensure `path` points to a valid NUL-terminated string.
pub unsafe fn truncate(path: *const u8, len: i64) -> i64 {
    syscall2(SYS_TRUNCATE, path as u64, len as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn ftruncate(fd: i64, len: i64) -> i64 {
    syscall2(SYS_FTRUNCATE, fd as u64, len as u64)
}
/// # Safety
/// Caller must uphold the kernel syscall ABI; no pointer arguments.
pub unsafe fn sys_getppid() -> i64 {
    syscall0(SYS_GETPPID)
}
