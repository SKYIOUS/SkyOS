#![no_std]
#![no_main]
extern crate alloc;

use libsarga::{sarga_main, println};

mod raw {
    pub fn open(path: *const u8, flags: i32, mode: u32) -> i64 {
        unsafe { libsarga::syscall::syscall3(2, path as u64, flags as u64, mode as u64) }
    }
    pub fn close(fd: i64) -> i64 {
        unsafe { libsarga::syscall::syscall1(3, fd as u64) }
    }
    pub fn unlink(path: *const u8) -> i64 {
        unsafe { libsarga::syscall::syscall1(87, path as u64) }
    }
    pub fn chmod(path: *const u8, mode: u32) -> i64 {
        unsafe { libsarga::syscall::syscall2(90, path as u64, mode as u64) }
    }
    pub fn chown(path: *const u8, uid: u32, gid: u32) -> i64 {
        unsafe { libsarga::syscall::syscall3(92, path as u64, uid as u64, gid as u64) }
    }
    pub fn umask(mask: u32) -> u64 {
        unsafe { libsarga::syscall::syscall1(95, mask as u64) as u64 }
    }
    pub fn access(path: *const u8, mode: i32) -> i64 {
        unsafe { libsarga::syscall::syscall2(21, path as u64, mode as u64) }
    }
    pub fn stat(path: *const u8, buf: *mut u8) -> i64 {
        unsafe { libsarga::syscall::syscall2(4, path as u64, buf as u64) }
    }
}

const O_RDONLY: i32 = 0;
const O_WRONLY: i32 = 1;
const O_RDWR: i32 = 2;
const O_CREAT: i32 = 0x40;

const R_OK: i32 = 4;
const W_OK: i32 = 2;

fn stat_mode(path: &str) -> i64 {
    let mut st: [u64; 11] = [0; 11];
    let mut buf = [0u8; 260];
    let bytes = path.as_bytes();
    buf[..bytes.len()].copy_from_slice(bytes);
    buf[bytes.len()] = 0;
    if raw::stat(buf.as_ptr(), st.as_mut_ptr() as *mut u8) < 0 { return -1; }
    st[2] as i64 // st_mode is 3rd field (offset 2)
}

fn main_test() -> i32 {
    let mut failed = 0;

    // Test 1: file creation with explicit mode, read back via stat
    println!("Test 1: create file with mode 0640, verify mode via stat");
    let mut buf = [0u8; 260];
    buf[..19].copy_from_slice(b"/tmp_dac_test1.txt\0");
    let fd = raw::open(buf.as_ptr(), O_CREAT | O_RDWR, 0o640);
    if fd < 0 { println!("FAIL: create file"); return 1; }
    raw::close(fd);

    let mode = stat_mode("/tmp_dac_test1.txt");
    let perms = mode & 0o777;
    if perms == 0o640 {
        println!("  PASS: mode = {:o}", perms);
    } else {
        println!("  FAIL: expected 0640, got {:o}", perms);
        failed += 1;
    }

    // Test 2: chmod to 0600
    println!("Test 2: chmod to 0600");
    let r = raw::chmod("/tmp_dac_test1.txt\0".as_ptr(), 0o600);
    if r < 0 { println!("  FAIL: chmod returned {}", r); failed += 1; }
    else {
        let mode = stat_mode("/tmp_dac_test1.txt");
        let perms = mode & 0o777;
        if perms == 0o600 {
            println!("  PASS: mode now {:o}", perms);
        } else {
            println!("  FAIL: expected 0600, got {:o}", perms);
            failed += 1;
        }
    }

    // Test 3: open for write after chmod 0444 (read-only) should fail
    println!("Test 3: open(O_WRONLY) on read-only file");
    if raw::chmod("/tmp_dac_test1.txt\0".as_ptr(), 0o444) < 0 {
        println!("  FAIL: chmod"); failed += 1;
    } else {
        let wfd = raw::open("/tmp_dac_test1.txt\0".as_ptr(), O_WRONLY, 0);
        if wfd < 0 {
            println!("  PASS: open write denied (EACCES)");
        } else {
            raw::close(wfd);
            println!("  FAIL: open write should have failed");
            failed += 1;
        }
    }

    // Test 4: but root (uid 0) can always read
    println!("Test 4: root can open 0444 for read");
    let rfd = raw::open("/tmp_dac_test1.txt\0".as_ptr(), O_RDONLY, 0);
    if rfd >= 0 {
        println!("  PASS: root can read");
        raw::close(rfd);
    } else {
        println!("  FAIL: root should be able to read");
        failed += 1;
    }

    // Test 5: umask affects created file mode
    println!("Test 5: umask 0077, create file, check mode");
    let old = raw::umask(0o077);
    let mut buf2 = [0u8; 260];
    buf2[..19].copy_from_slice(b"/tmp_dac_test2.txt\0");
    let fd2 = raw::open(buf2.as_ptr(), O_CREAT | O_RDWR, 0o666);
    if fd2 < 0 { println!("  FAIL: create file 2"); failed += 1; }
    else {
        raw::close(fd2);
        let mode2 = stat_mode("/tmp_dac_test2.txt");
        let perms2 = mode2 & 0o777;
        // umask 0077 should zero out group+other bits from 0666 → 0600
        if perms2 == 0o600 {
            println!("  PASS: mode = {:o}", perms2);
        } else {
            println!("  FAIL: expected 0600, got {:o}", perms2);
            failed += 1;
        }
        raw::unlink("/tmp_dac_test2.txt\0".as_ptr());
    }
    raw::umask(old as u32);

    // Test 6: chown (root only test — we're uid 0 by default)
    println!("Test 6: chown to uid=1,gid=1");
    let r = raw::chown("/tmp_dac_test1.txt\0".as_ptr(), 1, 1);
    if r < 0 { println!("  FAIL: chown {}", r); failed += 1; }
    else { println!("  PASS: chown succeeded"); }

    // Test 7: access() syscall
    println!("Test 7: access() on 0444 file");
    if raw::chmod("/tmp_dac_test1.txt\0".as_ptr(), 0o444) < 0 {
        println!("  FAIL: chmod"); failed += 1;
    } else {
        let r_ok = raw::access("/tmp_dac_test1.txt\0".as_ptr(), R_OK);
        let w_ok = raw::access("/tmp_dac_test1.txt\0".as_ptr(), W_OK);
        println!("  access R_OK = {} (0=ok), W_OK = {} (0=ok)", r_ok, w_ok);
        if r_ok == 0 && w_ok < 0 {
            println!("  PASS: read allowed, write denied");
        } else {
            println!("  FAIL: unexpected access results");
            failed += 1;
        }
    }

    // Cleanup
    raw::unlink("/tmp_dac_test1.txt\0".as_ptr());

    if failed == 0 {
        println!("PASS: all DAC tests passed");
        0
    } else {
        println!("FAIL: {} tests failed", failed);
        1
    }
}

fn user_main() -> i32 { main_test() }
sarga_main!(user_main);
