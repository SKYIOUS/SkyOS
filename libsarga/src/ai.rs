const SYS_VAHIAI: u64 = 300;

pub fn query(prompt: &str) -> Result<alloc::string::String, i64> {
    let mut out = [0u8; 2048];
    let n = unsafe {
        crate::syscall::syscall6(
            SYS_VAHIAI,
            prompt.as_ptr() as u64, prompt.len() as u64,
            out.as_mut_ptr() as u64, out.len() as u64, 0, 0,
        )
    };
    if n < 0 { return Err(-n); }
    Ok(alloc::string::String::from(core::str::from_utf8(&out[..n as usize])
        .unwrap_or("[SARGAAI response not valid UTF-8]")))
}
