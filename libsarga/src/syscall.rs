//! Raw x86_64 SYSCALL wrappers.
//! These are the ONLY place in Sarga OS that uses inline asm for syscalls.

#[inline(always)]
pub unsafe fn syscall6(n: u64, a1: u64, a2: u64, a3: u64,
                       a4: u64, a5: u64, a6: u64) -> i64 {
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

#[inline(always)] pub unsafe fn syscall0(n: u64) -> i64 { syscall6(n,0,0,0,0,0,0) }
#[inline(always)] pub unsafe fn syscall1(n: u64, a1: u64) -> i64 { syscall6(n,a1,0,0,0,0,0) }
#[inline(always)] pub unsafe fn syscall2(n: u64, a1: u64, a2: u64) -> i64 { syscall6(n,a1,a2,0,0,0,0) }
#[inline(always)] pub unsafe fn syscall3(n: u64, a1: u64, a2: u64, a3: u64) -> i64 { syscall6(n,a1,a2,a3,0,0,0) }
#[inline(always)] pub unsafe fn syscall4(n: u64, a1: u64, a2: u64, a3: u64, a4: u64) -> i64 { syscall6(n,a1,a2,a3,a4,0,0) }
#[inline(always)] pub unsafe fn syscall5(n: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64) -> i64 { syscall6(n,a1,a2,a3,a4,a5,0) }
