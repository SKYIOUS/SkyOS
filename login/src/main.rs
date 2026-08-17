#![no_std]
#![no_main]

extern crate alloc;
extern crate libsarga;

use alloc::string::ToString;
use alloc::vec::Vec;
use libsarga::errno::Error;
use libsarga::io::{self, close, ioctls, open, read};
use libsarga::process::{execve, setgid, setuid};
use libsarga::sarga_main;

const PASSWD_PATH: &str = "/etc/passwd";
const SHADOW_PATH: &str = "/etc/shadow";

/// Failed login attempts tolerated before the getty pauses. The getty still
/// re-prompts afterwards (never exits), so init's MAX_RESPAWNS accounting is
/// untouched — the cap only throttles the PBKDF2 verify (10k iterations per
/// attempt) so a brute-forcer or a stuck terminal cannot hammer it at full
/// speed.
const MAX_FAILED_ATTEMPTS: u32 = 10;
/// Backoff pause in nanoseconds after MAX_FAILED_ATTEMPTS (30 s).
const BACKOFF_NS: u64 = 30_000_000_000;

/// Count one failed login attempt. When the cap is reached, announce the
/// pause, sleep BACKOFF_NS, and reset the counter — the loop then re-prompts
/// as before. Never exits, by design: the console getty stays alive for
/// mistypes (the MAX_RESPAWNS fix depends on that).
fn note_failed_attempt(failures: &mut u32) {
    *failures += 1;
    if *failures >= MAX_FAILED_ATTEMPTS {
        io::print_str("\nToo many failed attempts - pausing 30s\n");
        // Only disarm the counter when the pause actually happened: a failed
        // nanosleep (e.g. EINTR) must not skip the backoff and arm the next
        // burst at full speed.
        if io::nanosleep(BACKOFF_NS).is_ok() {
            *failures = 0;
        }
    }
}

/// ECHO flag bit in termios `c_lflag` (POSIX).
///
/// NOTE: the kernel's `sys_ioctl` advertises `c_lflag: 0x5` with the comment
/// "ICANON | ECHO", but 0x5 is ISIG|ICANON per POSIX — ECHO (0x8) is not
/// actually set in the advertised value. We clear the POSIX ECHO bit; when
/// the kernel lands a real termios implementation, verify it uses POSIX
/// values (ECHO = 0x8), or this clear silently no-ops.
const ECHO: u32 = 0x8;

/// Termios layout mirrored from the kernel's `sys_ioctl` (repr(C), 4 u32
/// fields + c_cc). `c_lflag` is the only field we touch.
#[repr(C)]
struct Termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_cc: [u8; 19],
}

/// Disable input echo on `fd` (TCSETS clear ECHO) so a password typed at the
/// console is not echoed back onto the wire/log. Best-effort: the kernel's
/// TCSETS is currently a no-op returning 0, so this is forward-compatible
/// with a real termios implementation.
///
/// Returns the previous `c_lflag` on success (`Some`) so the caller can
/// restore it after reading; `None` when TCGETS/TCSETS failed (e.g. fd is
/// not a tty) — the caller must then skip the restore so a bogus 0 cannot
/// clobber real termios once the kernel implements TCSETS.
fn echo_off(fd: i64) -> Option<u32> {
    let mut t = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_cc: [0; 19],
    };
    // TCGETS first so the other fields (iflag/oflag/cflag) are preserved
    // when we write back — a TCSETS of a zeroed struct would clobber flow
    // control / canonical flags once the kernel implements it.
    if libsarga::io::ioctl(fd, ioctls::TCGETS, &mut t as *mut _ as *mut u8).is_err() {
        return None;
    }
    let saved = t.c_lflag;
    t.c_lflag &= !ECHO;
    if libsarga::io::ioctl(fd, ioctls::TCSETS, &mut t as *mut _ as *mut u8).is_err() {
        return None;
    }
    Some(saved)
}

/// Restore input echo on `fd` to `lflag` (TCSETS). Best-effort; reads the
/// current termios first so untouched fields are preserved.
fn echo_on(fd: i64, lflag: u32) {
    let mut t = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_cc: [0; 19],
    };
    if libsarga::io::ioctl(fd, ioctls::TCGETS, &mut t as *mut _ as *mut u8).is_err() {
        return;
    }
    t.c_lflag = lflag;
    let _ = libsarga::io::ioctl(fd, ioctls::TCSETS, &mut t as *mut _ as *mut u8);
}

/// Ensure the console tty echoes typed input (set the ECHO bit) before the
/// username read. The username is read BEFORE `echo_off` runs (only the
/// password is hidden), so it must not depend on the kernel's default
/// c_lflag having ECHO set — a prior TCSETS or a future non-echoing default
/// would otherwise leave the username invisible. Best-effort: silent no-op
/// if TCGETS/TCSETS fails (non-tty fd), mirroring echo_off/echo_on.
fn ensure_echo(fd: i64) {
    let mut t = Termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_cc: [0; 19],
    };
    if libsarga::io::ioctl(fd, ioctls::TCGETS, &mut t as *mut _ as *mut u8).is_err() {
        return;
    }
    t.c_lflag |= ECHO;
    let _ = libsarga::io::ioctl(fd, ioctls::TCSETS, &mut t as *mut _ as *mut u8);
}

/// Read one line from `fd`, terminating on `\n` or `\r`. Returns `Ok(None)`
/// on EOF (zero bytes read), `Ok(Some(line))` for a terminated line (which
/// may be empty), `Err` on a read error.
fn read_line(fd: i64) -> Result<Option<Vec<u8>>, Error> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = read(fd, &mut byte)?;
        if n == 0 {
            return Ok(None);
        }
        if byte[0] == b'\n' || byte[0] == b'\r' {
            break;
        }
        buf.push(byte[0]);
    }
    Ok(Some(buf))
}

/// Read a password line from `fd` with input echo disabled for the duration.
/// Mirrors the classic getty/login pattern: disable echo, read, restore.
/// The restore is skipped when echo_off failed (non-tty fd), so a bogus
/// lflag can never clobber real termios.
fn read_password(fd: i64) -> Result<Option<Vec<u8>>, Error> {
    let saved = echo_off(fd);
    let r = read_line(fd);
    if let Some(lflag) = saved {
        echo_on(fd, lflag);
    }
    r
}

fn read_whole_file(path: &str) -> Result<Vec<u8>, Error> {
    let fd = open(path, 0)?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = read(fd, &mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }
    let _ = close(fd);
    Ok(buf)
}

fn lookup_user(username: &str) -> Option<(u32, u32, Vec<u8>, Vec<u8>)> {
    let data = read_whole_file(PASSWD_PATH).ok()?;
    for line in data.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(7, |&b| b == b':');
        let name = parts.next()?;
        if name == username.as_bytes() {
            let _pw_passwd = parts.next()?;
            let uid_str = parts.next()?;
            let gid_str = parts.next()?;
            let _gecos = parts.next()?;
            let home = parts.next()?;
            let shell = parts.next()?;
            let uid = core::str::from_utf8(uid_str).ok()?.parse::<u32>().ok()?;
            let gid = core::str::from_utf8(gid_str).ok()?.parse::<u32>().ok()?;
            return Some((uid, gid, home.to_vec(), shell.to_vec()));
        }
    }
    None
}

fn verify_password(username: &str, password: &str) -> bool {
    let data = match read_whole_file(SHADOW_PATH) {
        Ok(d) => d,
        Err(_) => return false,
    };
    libsarga::hash::verify_password(&data, username, password)
}

fn user_main() -> i32 {
    let argc = libsarga::args::argc();
    // `login <user>` passes a fixed username on argv; the console getty (no
    // argv) prompts for both. Non-interactive invocations keep the old
    // exit-on-failure semantics: no caller exists today, and looping a
    // scripted call would hang it.
    let fixed_user = if argc > 1 {
        Some(libsarga::args::get(1).unwrap_or("root").to_string())
    } else {
        None
    };

    // Boot-time memory-pressure marker (evidence for
    // kernel-gui-window-fix.md Option 1 vs Option 2): read the kernel
    // buddy allocator's live free-page count from ctlFS at getty startup,
    // so the shell-interaction harness (qemu_shell_test.exp) collects the
    // same OOM evidence the GUI gate's login-manager does. The getty never
    // exits (mistype re-prompt loop below), so this prints once per boot,
    // not per respawn. Same ctlFS node and print shape as login-manager
    // (login-manager/src/main.rs:53-59) - the harnesses grep the same
    // '[login] mem free=' prefix.
    match libsarga::fs::read_to_string("/ctl/sys/mem/free") {
        Ok(free) => {
            let pages = free.trim().split(' ').next().unwrap_or("?");
            io::print_str(&alloc::format!("[login] mem free={} pages\n", pages));
        }
        Err(_) => io::print_str("[login] mem free=unavailable\n"),
    }

    // Interactive getty path: loop on bad credentials so a mistype
    // re-prompts instead of exiting. Exiting would make init respawn the
    // getty, and five consecutive exits would exhaust MAX_RESPAWNS and kill
    // the console login until reboot. `failures` tracks the attempt cap:
    // after MAX_FAILED_ATTEMPTS the loop pauses (BACKOFF_NS) but still never
    // exits, so the getty outlives any brute-force or stuck-terminal burst.
    let mut failures: u32 = 0;
    loop {
        let username = match fixed_user.as_deref() {
            Some(u) => u.to_string(),
            None => {
                io::print_str("login: ");
                // The username echoes (unlike the password below): set ECHO
                // explicitly instead of relying on the kernel's 0xB default.
                ensure_echo(0);
                let name_bytes = match read_line(0) {
                    Ok(Some(b)) => b,
                    Ok(None) | Err(_) => libsarga::process::exit(1),
                };
                if name_bytes.is_empty() {
                    // Bare Enter: re-prompt instead of exiting, so a mistype
                    // can't burn an init respawn (real EOF returns Ok(None)).
                    continue;
                }
                core::str::from_utf8(&name_bytes)
                    .unwrap_or("root")
                    .to_string()
            }
        };

        let (uid, gid, _home, _shell) = match lookup_user(&username) {
            Some(v) => v,
            None => {
                io::print_str(
                    "login: unknown user
",
                );
                if fixed_user.is_some() {
                    return 1;
                }
                note_failed_attempt(&mut failures);
                continue;
            }
        };

        io::print_str("Password: ");
        let pw_bytes = match read_password(0) {
            Ok(Some(b)) => b,
            Ok(None) | Err(_) => libsarga::process::exit(1),
        };

        let password = match core::str::from_utf8(&pw_bytes) {
            Ok(s) => s.to_string(),
            Err(_) => {
                io::print_str(
                    "
Invalid password encoding
",
                );
                if fixed_user.is_some() {
                    return 1;
                }
                note_failed_attempt(&mut failures);
                continue;
            }
        };

        if !verify_password(&username, &password) {
            io::print_str(
                "
Login incorrect
",
            );
            if fixed_user.is_some() {
                return 1;
            }
            note_failed_attempt(&mut failures);
            continue;
        }

        io::print_str(
            "
",
        );
        let _ = setuid(uid as u64);
        let _ = setgid(gid as u64);

        let shell_name = core::str::from_utf8(&_shell).unwrap_or("/bin/sash");
        let home_dir = core::str::from_utf8(&_home).unwrap_or("/");

        let env = [
            alloc::format!("HOME={}", home_dir),
            alloc::format!("USER={}", username),
            alloc::format!("LOGNAME={}", username),
            alloc::format!("SHELL={}", shell_name),
            "TERM=xterm-256color".to_string(),
        ];
        let env_refs: Vec<&str> = env
            .iter()
            .map(|s: &alloc::string::String| s.as_str())
            .collect();

        // Pass argv[0] = the shell name: an empty argv makes argv/opt scans
        // misbehave (init's spawn comment documents the same fix). execve
        // only returns on failure — behave like the old exit path.
        let _ = execve(shell_name, &[shell_name], &env_refs);
        return 1;
    }
}
sarga_main!(user_main);
