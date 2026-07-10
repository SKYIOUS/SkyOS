use crate::io;
use crate::errno;
use alloc::boxed::Box;

pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;

#[derive(Clone, Copy)]
#[repr(C)]
pub struct FILE {
    fd: i64,
    eof: bool,
    error: bool,
    unbuf: bool,
    pub _buffer: [u8; 128],
    pub _bufpos: usize,
    pub _bufsize: usize,
}

pub static mut STDIN: FILE = FILE { fd: 0, eof: false, error: false, unbuf: true, _buffer: [0; 128], _bufpos: 0, _bufsize: 0 };
pub static mut STDOUT: FILE = FILE { fd: 1, eof: false, error: false, unbuf: true, _buffer: [0; 128], _bufpos: 0, _bufsize: 0 };
pub static mut STDERR: FILE = FILE { fd: 2, eof: false, error: false, unbuf: true, _buffer: [0; 128], _bufpos: 0, _bufsize: 0 };

pub fn stdin() -> &'static mut FILE { unsafe { &mut *core::ptr::addr_of_mut!(STDIN) } }
pub fn stdout() -> &'static mut FILE { unsafe { &mut *core::ptr::addr_of_mut!(STDOUT) } }
pub fn stderr() -> &'static mut FILE { unsafe { &mut *core::ptr::addr_of_mut!(STDERR) } }

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

pub fn fclose(file: &mut FILE) -> i32 {
    if file.fd > 2 {
        match io::close(file.fd) {
            Ok(_) => { file.fd = -1; 0 }
            Err(_) => { file.error = true; -1 }
        }
    } else { 0 }
}

pub fn fread(buf: &mut [u8], file: &mut FILE) -> usize {
    match io::read(file.fd, buf) {
        Ok(0) => { file.eof = true; 0 }
        Ok(n) => n,
        Err(_) => { file.error = true; 0 }
    }
}

pub fn fwrite(buf: &[u8], file: &mut FILE) -> usize {
    match io::write_all(file.fd, buf) {
        Ok(_) => buf.len(),
        Err(_) => { file.error = true; 0 }
    }
}

pub fn fputs(s: &str, file: &mut FILE) -> i32 {
    fwrite(s.as_bytes(), file) as i32
}

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

#[macro_export]
macro_rules! fprintf {
    ($file:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {
        $crate::stdio::fprintf($file, core::format_args!($fmt $(, $arg)*))
    };
}

pub fn fgetc(file: &mut FILE) -> i32 {
    let mut c = [0u8; 1];
    match io::read(file.fd, &mut c) {
        Ok(0) => { file.eof = true; -1 }
        Ok(_) => c[0] as i32,
        Err(_) => { file.error = true; -1 }
    }
}

pub fn feof(file: &FILE) -> bool { file.eof }
pub fn ferror(file: &FILE) -> bool { file.error }
pub fn fileno(file: &FILE) -> i64 { file.fd }
