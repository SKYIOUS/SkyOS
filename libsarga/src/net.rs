use crate::syscall::*;

pub const AF_INET: u64 = 2;
pub const SOCK_STREAM: u64 = 1;
pub const SOCK_DGRAM: u64 = 2;
pub const SOCK_RAW: u64 = 3;

pub fn socket(domain: u64, ty: u64, protocol: u64) -> Result<i64, i64> {
    let r = unsafe { syscall3(41, domain, ty, protocol) };
    if r < 0 { Err(-r) } else { Ok(r) }
}

pub fn connect(sockfd: i64, addr: &[u8]) -> Result<(), i64> {
    let r = unsafe { syscall3(42, sockfd as u64, addr.as_ptr() as u64, addr.len() as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn bind(sockfd: i64, addr: &[u8]) -> Result<(), i64> {
    let r = unsafe { syscall3(49, sockfd as u64, addr.as_ptr() as u64, addr.len() as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn listen(sockfd: i64, backlog: u64) -> Result<(), i64> {
    let r = unsafe { syscall2(50, sockfd as u64, backlog) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub fn accept(sockfd: i64, addr: &mut [u8], addrlen: &mut u32) -> Result<i64, i64> {
    let r = unsafe { syscall3(43, sockfd as u64, addr.as_mut_ptr() as u64, addrlen as *mut u32 as u64) };
    if r < 0 { Err(-r) } else { Ok(r) }
}

pub fn sendto(sockfd: i64, buf: &[u8], addr: Option<&[u8]>) -> Result<usize, i64> {
    let (aptr, alen) = match addr {
        Some(a) => (a.as_ptr() as u64, a.len() as u64),
        None => (0, 0),
    };
    let r = unsafe { syscall5(44, sockfd as u64, buf.as_ptr() as u64, buf.len() as u64, aptr, alen) };
    if r < 0 { Err(-r) } else { Ok(r as usize) }
}

pub fn recvfrom(sockfd: i64, buf: &mut [u8]) -> Result<(usize, Option<[u8; 16]>), i64> {
    let mut addr_buf = [0u8; 16];
    let mut addr_len: u32 = 16;
    let r = unsafe { syscall5(45, sockfd as u64, buf.as_mut_ptr() as u64, buf.len() as u64,
                              addr_buf.as_mut_ptr() as u64, (&mut addr_len) as *mut u32 as u64) };
    if r < 0 { Err(-r) } else {
        if addr_len > 0 { Ok((r as usize, Some(addr_buf))) }
        else { Ok((r as usize, None)) }
    }
}

pub fn send(sockfd: i64, buf: &[u8]) -> Result<usize, i64> {
    sendto(sockfd, buf, None)
}

pub fn recv(sockfd: i64, buf: &mut [u8]) -> Result<usize, i64> {
    let (n, _) = recvfrom(sockfd, buf)?;
    Ok(n)
}

pub fn close(sockfd: i64) -> Result<(), i64> {
    let r = unsafe { syscall1(3, sockfd as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

#[repr(C, packed)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: [u8; 4],
    pub sin_zero: [u8; 8],
}

impl SockAddrIn {
    pub fn new(ip: [u8; 4], port: u16) -> Self {
        SockAddrIn {
            sin_family: AF_INET as u16,
            sin_port: port.to_be(),
            sin_addr: ip,
            sin_zero: [0u8; 8],
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts((self as *const Self) as *const u8, core::mem::size_of::<Self>()) }
    }
}

pub fn resolve(hostname: &str, ip_out: &mut [u8; 4]) -> Result<(), i64> {
    let r = unsafe { crate::syscall::syscall2(200, hostname.as_ptr() as u64, ip_out.as_mut_ptr() as u64) };
    if r < 0 { Err(-r) } else { Ok(()) }
}

pub struct HttpClient;

impl HttpClient {
    pub fn get(url: &str) -> Result<alloc::vec::Vec<u8>, i64> {
        // Simple URL parser (only http:// supported)
        if !url.starts_with("http://") { return Err(22); }
        let rest = &url[7..];
        let (host, path) = match rest.find('/') {
            Some(pos) => (&rest[..pos], &rest[pos..]),
            None => (rest, "/"),
        };

        let mut ip = [0u8; 4];
        resolve(host, &mut ip)?;

        let fd = socket(AF_INET, SOCK_STREAM, 0)?;
        let addr = SockAddrIn::new(ip, 80);
        connect(fd, addr.as_bytes())?;

        let request = alloc::format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, host);
        send(fd, request.as_bytes())?;

        let mut response = alloc::vec::Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = recv(fd, &mut buf)?;
            if n == 0 { break; }
            response.extend_from_slice(&buf[..n]);
        }
        close(fd)?;

        // Find end of headers
        if let Some(pos) = response.windows(4).position(|w| w == b"\r\n\r\n") {
            Ok(response[pos + 4..].to_vec())
        } else {
            Ok(response)
        }
    }
}
