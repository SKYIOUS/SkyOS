//! Error handling and standard error codes.

use core::sync::atomic::{AtomicI32, Ordering};

static __ERRNO: AtomicI32 = AtomicI32::new(0);

/// Returns a pointer to the thread-local errno location.
#[no_mangle]
pub extern "C" fn __errno_location() -> *mut i32 {
    __ERRNO.as_ptr() as *mut i32
}

/// Sets the current thread's error number.
pub fn set_errno(err: i32) {
    __ERRNO.store(err, Ordering::SeqCst);
}

/// Gets the current thread's error number.
pub fn get_errno() -> i32 {
    __ERRNO.load(Ordering::SeqCst)
}

/// Standard Sarga OS Error enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Error {
    EPERM = 1,
    ENOENT = 2,
    ESRCH = 3,
    EINTR = 4,
    EIO = 5,
    ENXIO = 6,
    E2BIG = 7,
    ENOEXEC = 8,
    EBADF = 9,
    ECHILD = 10,
    EAGAIN = 11,
    ENOMEM = 12,
    EACCES = 13,
    EFAULT = 14,
    ENOTBLK = 15,
    EBUSY = 16,
    EEXIST = 17,
    EXDEV = 18,
    ENODEV = 19,
    ENOTDIR = 20,
    EISDIR = 21,
    EINVAL = 22,
    ENFILE = 23,
    EMFILE = 24,
    ENOTTY = 25,
    ETXTBSY = 26,
    EFBIG = 27,
    ENOSPC = 28,
    ESPIPE = 29,
    EROFS = 30,
    EMLINK = 31,
    EPIPE = 32,
    EDOM = 33,
    ERANGE = 34,
    ENOSYS = 38,
    EAFNOSUPPORT = 97,
    EADDRINUSE = 98,
    EUNKNOWN = 1000,
}

impl Error {
    /// Converts a raw i64 return value from a syscall into an Error.
    pub fn from_i64(err: i64) -> Self {
        match -(err as i32) {
            1 => Error::EPERM,
            2 => Error::ENOENT,
            3 => Error::ESRCH,
            4 => Error::EINTR,
            5 => Error::EIO,
            6 => Error::ENXIO,
            7 => Error::E2BIG,
            8 => Error::ENOEXEC,
            9 => Error::EBADF,
            10 => Error::ECHILD,
            11 => Error::EAGAIN,
            12 => Error::ENOMEM,
            13 => Error::EACCES,
            14 => Error::EFAULT,
            15 => Error::ENOTBLK,
            16 => Error::EBUSY,
            17 => Error::EEXIST,
            18 => Error::EXDEV,
            19 => Error::ENODEV,
            20 => Error::ENOTDIR,
            21 => Error::EISDIR,
            22 => Error::EINVAL,
            23 => Error::ENFILE,
            24 => Error::EMFILE,
            25 => Error::ENOTTY,
            26 => Error::ETXTBSY,
            27 => Error::EFBIG,
            28 => Error::ENOSPC,
            29 => Error::ESPIPE,
            30 => Error::EROFS,
            31 => Error::EMLINK,
            32 => Error::EPIPE,
            33 => Error::EDOM,
            34 => Error::ERANGE,
            38 => Error::ENOSYS,
            97 => Error::EAFNOSUPPORT,
            98 => Error::EADDRINUSE,
            _ => Error::EUNKNOWN,
        }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub const EPERM: i32 = 1;
pub const ENOENT: i32 = 2;
pub const ESRCH: i32 = 3;
pub const EINTR: i32 = 4;
pub const EIO: i32 = 5;
pub const ENXIO: i32 = 6;
pub const E2BIG: i32 = 7;
pub const ENOEXEC: i32 = 8;
pub const EBADF: i32 = 9;
pub const ECHILD: i32 = 10;
pub const EAGAIN: i32 = 11;
pub const ENOMEM: i32 = 12;
pub const EACCES: i32 = 13;
pub const EFAULT: i32 = 14;
pub const ENOTBLK: i32 = 15;
pub const EBUSY: i32 = 16;
pub const EEXIST: i32 = 17;
pub const EXDEV: i32 = 18;
pub const ENODEV: i32 = 19;
pub const ENOTDIR: i32 = 20;
pub const EISDIR: i32 = 21;
pub const EINVAL: i32 = 22;
pub const ENFILE: i32 = 23;
pub const EMFILE: i32 = 24;
pub const ENOTTY: i32 = 25;
pub const ETXTBSY: i32 = 26;
pub const EFBIG: i32 = 27;
pub const ENOSPC: i32 = 28;
pub const ESPIPE: i32 = 29;
pub const EROFS: i32 = 30;
pub const EMLINK: i32 = 31;
pub const EPIPE: i32 = 32;
pub const EDOM: i32 = 33;
pub const ERANGE: i32 = 34;
pub const ENOSYS: i32 = 38;
pub const EAFNOSUPPORT: i32 = 97;
pub const EADDRINUSE: i32 = 98;
