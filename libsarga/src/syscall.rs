//! Raw x86_64 SYSCALL wrappers.
//! These are the ONLY place in Sarga OS that uses inline asm for syscalls.

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

#[inline(always)]
pub unsafe fn syscall0(n: u64) -> i64 {
    syscall6(n, 0, 0, 0, 0, 0, 0)
}
#[inline(always)]
pub unsafe fn syscall1(n: u64, a1: u64) -> i64 {
    syscall6(n, a1, 0, 0, 0, 0, 0)
}
#[inline(always)]
pub unsafe fn syscall2(n: u64, a1: u64, a2: u64) -> i64 {
    syscall6(n, a1, a2, 0, 0, 0, 0)
}
#[inline(always)]
pub unsafe fn syscall3(n: u64, a1: u64, a2: u64, a3: u64) -> i64 {
    syscall6(n, a1, a2, a3, 0, 0, 0)
}
#[inline(always)]
pub unsafe fn syscall4(n: u64, a1: u64, a2: u64, a3: u64, a4: u64) -> i64 {
    syscall6(n, a1, a2, a3, a4, 0, 0)
}
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
pub const SYS_FCHMOD: u64 = 91;
pub const SYS_FCHOWN: u64 = 93;
pub const SYS_GETDENTS64: u64 = 217;
pub const SYS_SCHED_YIELD: u64 = 24;
pub const SYS_CLOCK_GETTIME: u64 = 228;
pub const SYS_FUTEX: u64 = 202;
pub const SYS_MOUNT: u64 = 165;
pub const SYS_UMOUNT2: u64 = 166;
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

pub unsafe fn read(fd: i64, buf: *mut u8, len: usize) -> i64 {
    syscall3(SYS_READ, fd as u64, buf as u64, len as u64)
}
pub unsafe fn write(fd: i64, buf: *const u8, len: usize) -> i64 {
    syscall3(SYS_WRITE, fd as u64, buf as u64, len as u64)
}
pub unsafe fn open(path: *const u8, flags: i32) -> i64 {
    syscall2(SYS_OPEN, path as u64, flags as u64)
}
pub unsafe fn close(fd: i64) -> i64 {
    syscall1(SYS_CLOSE, fd as u64)
}
pub unsafe fn fork() -> i64 {
    syscall0(SYS_FORK)
}
pub unsafe fn execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> i64 {
    syscall3(SYS_EXECVE, path as u64, argv as u64, envp as u64)
}
pub unsafe fn exit(code: i32) -> ! {
    syscall1(SYS_EXIT, code as u64);
    loop {}
}
pub unsafe fn wait4(pid: i64, status: *mut i32, options: i32, rusage: *mut u8) -> i64 {
    syscall4(
        SYS_WAIT4,
        pid as u64,
        status as u64,
        options as u64,
        rusage as u64,
    )
}
pub unsafe fn getdents64(fd: i64, buf: *mut u8, len: usize) -> i64 {
    syscall3(SYS_GETDENTS64, fd as u64, buf as u64, len as u64)
}
pub unsafe fn unlink(path: *const u8) -> i64 {
    syscall1(SYS_UNLINK, path as u64)
}
pub unsafe fn mkdir(path: *const u8, mode: u32) -> i64 {
    syscall2(SYS_MKDIR, path as u64, mode as u64)
}
pub unsafe fn fstat(fd: i64, buf: *mut u8) -> i64 {
    syscall2(SYS_FSTAT, fd as u64, buf as u64)
}

pub unsafe fn beep(freq: u32, duration: u32) -> i64 {
    syscall2(SYS_BEEP, freq as u64, duration as u64)
}

// Futex operations
pub const FUTEX_WAIT: u32 = 0;
pub const FUTEX_WAKE: u32 = 1;

/// Futex syscall wrapper.
/// Matches the kernel's SYS_FUTEX (202) ABI:
///   uaddr, op, val, timeout, uaddr2, val3
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
pub unsafe fn sys_gettid() -> i64 {
    syscall0(SYS_GETTID)
}
