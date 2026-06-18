use alloc::string::String;
use core::str::from_utf8;

const SYS_VAHIAI: u64 = 300;

pub fn query(prompt: &str) -> Result<String, i64> {
    let mut out = [0u8; 4096];
    let n = unsafe {
        crate::syscall::syscall6(
            SYS_VAHIAI,
            prompt.as_ptr() as u64, prompt.len() as u64,
            out.as_mut_ptr() as u64, out.len() as u64, 0, 0,
        )
    };
    if n < 0 { return Err(-n); }
    let slice = &out[..n as usize];
    Ok(from_utf8(slice).unwrap_or("[SARGAAI response not valid UTF-8]").trim().into())
}

pub fn query_with_args(prompt: &str, args: &[&str]) -> Result<String, i64> {
    let mut full = String::from(prompt);
    for arg in args {
        full.push(' ');
        full.push_str(arg);
    }
    query(&full)
}

pub fn read_ctl_file(path: &str) -> Result<String, i64> {
    let mut full_path = String::from("/ctl");
    if !path.starts_with('/') { full_path.push('/'); }
    full_path.push_str(path);

    let fd = crate::io::open(&full_path, 0)?;
    let mut buf = [0u8; 2048];
    let n = crate::io::read(fd, &mut buf)?;
    let _ = crate::io::close(fd);
    Ok(from_utf8(&buf[..n]).unwrap_or("").trim().into())
}

pub fn process_list() -> Result<String, i64> {
    read_ctl_file("proc/list")
}

pub fn memory_info() -> Result<String, i64> {
    let total = read_ctl_file("sys/mem/total")?;
    let free = read_ctl_file("sys/mem/free")?;
    Ok(alloc::format!("Memory:\n  Total: {}\n  Free: {}", total.trim(), free.trim()))
}

pub fn cpu_info() -> Result<String, i64> {
    read_ctl_file("sys/cpu/info")
}

pub fn cpu_load() -> Result<String, i64> {
    read_ctl_file("sys/cpu/0/load")
}

pub fn kernel_version() -> Result<String, i64> {
    read_ctl_file("kernel/version")
}

pub fn uptime() -> Result<String, i64> {
    read_ctl_file("kernel/uptime")
}

pub fn kernel_log() -> Result<String, i64> {
    read_ctl_file("kernel/log")
}

pub fn system_status() -> Result<String, i64> {
    let mut msg = String::new();
    msg.push_str(&kernel_version()?);
    msg.push_str(&uptime()?);
    msg.push('\n');
    msg.push_str(&cpu_info()?);
    msg.push('\n');
    msg.push_str(&memory_info()?);
    msg.push('\n');
    match process_list() {
        Ok(p) => { msg.push_str("Processes:\n"); msg.push_str(&p); }
        Err(e) => { let _ = alloc::format!(" (process list: {})\n", e); }
    }
    Ok(msg)
}

pub fn handle_intent(input: &str) -> Result<String, i64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let lower = trimmed.to_lowercase();

    if lower.starts_with("ps") || lower.starts_with("process") || lower.starts_with("list proc") || lower == "procs" {
        return Ok(process_list()?);
    }
    if lower.starts_with("mem") || lower.starts_with("memory") || lower.starts_with("free") {
        return Ok(memory_info()?);
    }
    if lower.starts_with("cpu") || lower.starts_with("load") {
        return Ok(cpu_load()?);
    }
    if lower.starts_with("uptime") || lower.starts_with("up") {
        return Ok(uptime()?);
    }
    if lower.starts_with("version") || lower.starts_with("ver") || lower.starts_with("kernel") {
        return Ok(kernel_version()?);
    }
    if lower.starts_with("log") || lower.starts_with("dmesg") {
        return Ok(kernel_log()?);
    }
    if lower.starts_with("info") || lower.starts_with("status") || lower.starts_with("sysinfo") {
        return Ok(system_status()?);
    }
    if lower.starts_with("help") || lower == "?" || lower.starts_with("commands") {
        let help = "\
SARGAAI Commands:
  ps, procs, list proc   — Show process list
  mem, memory, free      — Show memory info
  cpu, load              — Show CPU load
  uptime, up             — Show system uptime
  version, ver, kernel   — Show kernel version
  log, dmesg             — Show kernel log
  info, status, sysinfo  — Show all system info
  help, ?                — Show this help
  <anything else>        — Send to kernel SARGAAI intent engine";
        return Ok(String::from(help));
    }

    query(trimmed)
}
