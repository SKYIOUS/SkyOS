//! Standard buffered I/O.

use crate::io;
use crate::errno;
use alloc::boxed::Box;

/// Seek from the start of the file.
pub const SEEK_SET: i32 = 0;
/// Seek from the current position.
pub const SEEK_CUR: i32 = 1;
/// Seek from the end of the file.
pub const SEEK_END: i32 = 2;

/// Standard file stream.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct FILE {
    fd: i64,
    eof: bool,
    error: bool,
    unbuf: bool,
    /// Internal buffer
    pub _buffer: [u8; 128],
    /// Current buffer position
    pub _bufpos: usize,
    /// Total bytes in buffer
    pub _bufsize: usize,
}

/// Standard input stream.
pub static mut STDIN: FILE = FILE { fd: 0, eof: false, error: false, unbuf: true, _buffer: [0; 128], _bufpos: 0, _bufsize: 0 };
/// Standard output stream.
pub static mut STDOUT: FILE = FILE { fd: 1, eof: false, error: false, unbuf: true, _buffer: [0; 128], _bufpos: 0, _bufsize: 0 };
/// Standard error stream.
pub static mut STDERR: FILE = FILE { fd: 2, eof: false, error: false, unbuf: true, _buffer: [0; 128], _bufpos: 0, _bufsize: 0 };

/// Returns a reference to the standard input stream.
pub fn stdin() -> &'static mut FILE { unsafe { &mut *core::ptr::addr_of_mut!(STDIN) } }
/// Returns a reference to the standard output stream.
pub fn stdout() -> &'static mut FILE { unsafe { &mut *core::ptr::addr_of_mut!(STDOUT) } }
/// Returns a reference to the standard error stream.
pub fn stderr() -> &'static mut FILE { unsafe { &mut *core::ptr::addr_of_mut!(STDERR) } }

/// Opens a file stream.
pub fn fopen(path: &str, mode: &str) -> Option<&'static mut FILE> {
    let flags = if mode.contains('w') {
        if mode.contains('+') { 0x42 } else { 0x41 }
    } else if mode.contains('a') {
        if mode.contains('+') { 0x42 } else { 0x401 }
    } else {
        if mode.contains('+') { 0x42 } else { 0x40 }
    };

    match io::open(path, flags) {
        Ok(fd) => {
            let f = Box::new(FILE {
                fd,
                eof: false,
                error: false,
                unbuf: true,
                _buffer: [0; 128],
                _bufpos: 0,
                _bufsize: 0,
            });
            Some(Box::leak(f))
        }
        Err(e) => {
            errno::set_errno(e as i32);
            None
        }
    }
}

/// Closes a file stream.
pub fn fclose(file: &mut FILE) -> i32 {
    if file.fd > 2 {
        match io::close(file.fd) {
            Ok(_) => { file.fd = -1; 0 }
            Err(_) => { file.error = true; -1 }
        }
    } else { 0 }
}

/// Reads data from a file stream.
pub fn fread(buf: &mut [u8], file: &mut FILE) -> usize {
    match io::read(file.fd, buf) {
        Ok(0) => { file.eof = true; 0 }
        Ok(n) => n,
        Err(_) => { file.error = true; 0 }
    }
}

/// Writes data to a file stream.
pub fn fwrite(buf: &[u8], file: &mut FILE) -> usize {
    match io::write_all(file.fd, buf) {
        Ok(_) => buf.len(),
        Err(_) => { file.error = true; 0 }
    }
}

/// Writes a string to a file stream.
pub fn fputs(s: &str, file: &mut FILE) -> i32 {
    fwrite(s.as_bytes(), file) as i32
}

/// Formats and prints data to a file stream.
pub fn fprintf(file: &mut FILE, args: core::fmt::Arguments<'_>) -> i32 {
    match core::fmt::write(&mut FwriteWriter(file), args) {
        Ok(_) => 0,
        Err(_) => { file.error = true; -1 }
    }
}

struct FwriteWriter<'a>(&'a mut FILE);

impl core::fmt::Write for FwriteWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if fwrite(s.as_bytes(), self.0) == s.len() { Ok(()) } else { Err(core::fmt::Error) }
    }
}

/// Reads a single character from a file stream.
pub fn fgetc(file: &mut FILE) -> i32 {
    let mut c = [0u8; 1];
    match io::read(file.fd, &mut c) {
        Ok(0) => { file.eof = true; -1 }
        Ok(_) => c[0] as i32,
        Err(_) => { file.error = true; -1 }
    }
}

/// Returns true if end-of-file has been reached on a stream.
pub fn feof(file: &FILE) -> bool { file.eof }
/// Returns true if an error has occurred on a stream.
pub fn ferror(file: &FILE) -> bool { file.error }
/// Returns the underlying file descriptor of a stream.
pub fn fileno(file: &FILE) -> i64 { file.fd }

/// Prints formatted data to standard output.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        {
            let _ = $crate::stdio::fprintf($crate::stdio::stdout(), format_args!($($arg)*));
        }
    };
}

/// Prints formatted data to standard output, followed by a newline.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {
        {
            let _ = $crate::stdio::fprintf($crate::stdio::stdout(), format_args!($($arg)*));
            let _ = $crate::stdio::fwrite(b"\n", $crate::stdio::stdout());
        }
    };
}
