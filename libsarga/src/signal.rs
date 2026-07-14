use crate::errno::Error;
use crate::syscall::*;

// Signal numbers matching kernel signal.rs
pub const SIGHUP: u32 = 1;
pub const SIGINT: u32 = 2;
pub const SIGQUIT: u32 = 3;
pub const SIGILL: u32 = 4;
pub const SIGTRAP: u32 = 5;
pub const SIGABRT: u32 = 6;
pub const SIGBUS: u32 = 7;
pub const SIGFPE: u32 = 8;
pub const SIGKILL: u32 = 9;
pub const SIGUSR1: u32 = 10;
pub const SIGSEGV: u32 = 11;
pub const SIGUSR2: u32 = 12;
pub const SIGPIPE: u32 = 13;
pub const SIGALRM: u32 = 14;
pub const SIGTERM: u32 = 15;
pub const SIGSTKFLT: u32 = 16;
pub const SIGCHLD: u32 = 17;
pub const SIGCONT: u32 = 18;
pub const SIGSTOP: u32 = 19;
pub const SIGTSTP: u32 = 20;
pub const SIGTTIN: u32 = 21;
pub const SIGTTOU: u32 = 22;
pub const SIGURG: u32 = 23;
pub const SIGXCPU: u32 = 24;
pub const SIGXFSZ: u32 = 25;
pub const SIGVTALRM: u32 = 26;
pub const SIGPROF: u32 = 27;
pub const SIGWINCH: u32 = 28;
pub const SIGIO: u32 = 29;
pub const SIGPWR: u32 = 30;
pub const SIGSYS: u32 = 31;

pub const SIG_DFL: u64 = 0;
pub const SIG_IGN: u64 = 1;
pub const SIG_ERR: u64 = !0u64;

// sa_flags
pub const SA_NOCLDSTOP: u64 = 1;
pub const SA_NOCLDWAIT: u64 = 2;
pub const SA_SIGINFO: u64 = 4;
pub const SA_ONSTACK: u64 = 0x08000000;
pub const SA_RESTART: u64 = 0x10000000;
pub const SA_NODEFER: u64 = 0x40000000;
pub const SA_RESETHAND: u64 = 0x80000000;

// sigprocmask how
pub const SIG_BLOCK: i32 = 0;
pub const SIG_UNBLOCK: i32 = 1;
pub const SIG_SETMASK: i32 = 2;

/// Kernel ABI: struct sigaction { sa_handler(u64), sa_flags(u64), sa_restorer(u64), sa_mask(u64) }
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SigAction {
    pub sa_handler: u64,
    pub sa_flags: u64,
    pub sa_restorer: u64,
    pub sa_mask: u64,
}

impl SigAction {
    pub const fn default() -> Self {
        SigAction { sa_handler: 0, sa_flags: 0, sa_restorer: 0, sa_mask: 0 }
    }

    pub fn handler(handler: u64) -> Self {
        SigAction { sa_handler: handler, sa_flags: 0, sa_restorer: 0, sa_mask: 0 }
    }
}

pub type SigSet = u64;

pub fn sigemptyset() -> SigSet { 0 }
pub fn sigfillset() -> SigSet { !0u64 }
pub fn sigaddset(set: SigSet, sig: u32) -> SigSet {
    if sig == 0 || sig > 64 { return set; }
    set | (1u64 << (sig - 1))
}
pub fn sigdelset(set: SigSet, sig: u32) -> SigSet {
    if sig == 0 || sig > 64 { return set; }
    set & !(1u64 << (sig - 1))
}
pub fn sigismember(set: SigSet, sig: u32) -> bool {
    if sig == 0 || sig > 64 { return false; }
    (set & (1u64 << (sig - 1))) != 0
}

pub fn rt_sigaction(sig: u32, act: Option<&SigAction>, oldact: Option<&mut SigAction>) -> Result<(), Error> {
    let act_ptr = act.map(|a| a as *const SigAction as *const u64).unwrap_or(core::ptr::null());
    let old_ptr = oldact.map(|a| a as *mut SigAction as *mut u64).unwrap_or(core::ptr::null_mut());
    let r = unsafe { syscall4(SYS_RT_SIGACTION, sig as u64, act_ptr as u64, old_ptr as u64, 8) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

pub fn rt_sigprocmask(how: i32, set: Option<SigSet>, oldset: Option<&mut SigSet>) -> Result<(), Error> {
    let set_val = set.unwrap_or(0);
    let set_ptr = if set.is_some() { &set_val as *const u64 as u64 } else { 0 };
    let old_ptr = oldset.map(|o| o as *mut SigSet as u64).unwrap_or(0);
    let r = unsafe { syscall3(309, how as u64, set_ptr, old_ptr) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

pub fn kill(pid: i64, sig: u32) -> Result<(), Error> {
    crate::process::kill(pid, sig)
}

pub fn raise(sig: u32) -> Result<(), Error> {
    kill(0, sig)
}

pub fn signal(sig: u32, handler: u64) -> Result<u64, Error> {
    let old = SigAction::default();
    let mut old_mut = old;
    let act = SigAction::handler(handler);
    rt_sigaction(sig, Some(&act), Some(&mut old_mut))?;
    Ok(old_mut.sa_handler)
}

// ── Wait status inspection macros ─────────────────────────────────

/// True if child exited normally (via exit/_exit).
pub fn WIFEXITED(status: i32) -> bool {
    (status & 0x7f) == 0
}

/// Extract the exit code when WIFEXITED is true.
pub fn WEXITSTATUS(status: i32) -> i32 {
    (status >> 8) & 0xff
}

/// True if child was terminated by a signal.
pub fn WIFSIGNALED(status: i32) -> bool {
    ((status & 0x7f) != 0) && ((status & 0x7f) != 0x7f)
}

/// Extract the terminating signal number when WIFSIGNALED is true.
pub fn WTERMSIG(status: i32) -> u32 {
    status as u32 & 0x7f
}

/// True if child is currently stopped (via SIGSTOP/SIGTSTP).
pub fn WIFSTOPPED(status: i32) -> bool {
    (status & 0xff) == 0x7f
}

/// Extract the stop signal when WIFSTOPPED is true.
pub fn WSTOPSIG(status: i32) -> u32 {
    (status as u32 >> 8) & 0xff
}

pub fn WCOREDUMP(status: i32) -> bool {
    (status & 0x80) != 0
}

/// Generate a status value for a normal exit.
pub fn W_EXITCODE(code: i32, sig: i32) -> i32 {
    (code << 8) | sig
}

/// Generate a status value for a stop signal.
pub fn W_STOPCODE(sig: i32) -> i32 {
    (sig << 8) | 0x7f
}
