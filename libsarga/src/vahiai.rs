//! VahiAI integration for advanced kernel intelligence.

use alloc::string::String;
use core::str::from_utf8;
use crate::errno::Error;

/// System call number for VahiAI.
const SYS_VAHIAI: u64 = 300;

/// Queries the VahiAI engine.
pub fn query(prompt: &str) -> Result<String, Error> {
    let mut out = [0u8; 4096];
    // SAFETY: syscall is safe here
    let n = unsafe {
        crate::syscall::syscall6(
            SYS_VAHIAI,
            prompt.as_ptr() as u64, prompt.len() as u64,
            out.as_mut_ptr() as u64, out.len() as u64, 0, 0,
        )
    };
    if n < 0 { return Err(Error::from_i64(n)); }
    let slice = &out[..n as usize];
    Ok(from_utf8(slice).unwrap_or("[VahiAI response not valid UTF-8]").trim().into())
}

/// Helper to read kernel control files.
pub fn read_ctl_file(path: &str) -> Result<String, Error> {
    let mut full_path = String::from("/ctl");
    if !path.starts_with('/') { full_path.push('/'); }
    full_path.push_str(path);

    let fd = crate::io::open(&full_path, 0)?;
    let mut buf = [0u8; 2048];
    let n = crate::io::read(fd, &mut buf)?;
    let _ = crate::io::close(fd);
    Ok(from_utf8(&buf[..n]).unwrap_or("").trim().into())
}

/// Retrieves the system process list.
pub fn process_list() -> Result<String, Error> {
    read_ctl_file("proc/list")
}

/// Retrieves system memory information.
pub fn memory_info() -> Result<String, Error> {
    let total = read_ctl_file("sys/mem/total")?;
    let free = read_ctl_file("sys/mem/free")?;
    Ok(alloc::format!("Memory:\n  Total: {}\n  Free: {}", total.trim(), free.trim()))
}

/// Retrieves system status summary.
pub fn system_status() -> Result<String, Error> {
    let mut msg = String::new();
    msg.push_str(&read_ctl_file("kernel/version")?);
    msg.push_str(&read_ctl_file("kernel/uptime")?);
    msg.push('\n');
    msg.push_str(&memory_info()?);
    Ok(msg)
}

/// Handles a natural language intent by dispatching to system info or AI query.
pub fn handle_intent(input: &str) -> Result<String, Error> {
    let lower = input.to_lowercase();
    if lower.contains("proc") || lower.contains("process") {
        return process_list();
    }
    if lower.contains("mem") || lower.contains("memory") {
        return memory_info();
    }
    if lower.contains("status") || lower.contains("info") {
        return system_status();
    }
    query(input)
}
