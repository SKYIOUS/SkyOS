//! Networking operations and socket management.

use crate::errno::Error;
use alloc::vec::Vec;
use crate::io;

/// Socket domain.
#[repr(u64)]
pub enum SocketDomain {
    /// IPv4 internet protocols.
    Inet = 2,
    /// IPv6 internet protocols.
    Inet6 = 10,
}

/// Socket type.
#[repr(u64)]
pub enum SocketType {
    /// Reliable, connection-oriented byte streams.
    Stream = 1,
    /// Connectionless, unreliable datagrams.
    Datagram = 2,
    /// Raw network protocol access.
    Raw = 3,
}

/// Poll event flags.
pub const POLLIN: i16 = 0x0001;
/// Poll event flags.
pub const POLLOUT: i16 = 0x0004;
/// Poll event flags.
pub const POLLERR: i16 = 0x0008;

/// Poll file descriptor structure.
#[repr(C)]
pub struct PollFd {
    pub fd: i64,
    pub events: i16,
    pub revents: i16,
}

/// Waits for events on multiple file descriptors.
pub fn poll(fds: &mut [PollFd], timeout_ms: i32) -> Result<i32, Error> {
    // SAFETY: poll syscall is safe here
    let r = unsafe { crate::syscall::syscall3(7, fds.as_mut_ptr() as u64, fds.len() as u64, timeout_ms as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r as i32) }
}

/// IPv4 Socket Address structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: [u8; 4],
    pub sin_zero: [u8; 8],
}

impl SockAddrIn {
    pub fn new(ip: [u8; 4], port: u16) -> Self {
        Self {
            sin_family: 2, // AF_INET
            sin_port: port.to_be(),
            sin_addr: ip,
            sin_zero: [0; 8],
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, core::mem::size_of::<Self>()) }
    }
}

/// Resolves a hostname to an IPv4 address.
pub fn resolve(name: &str, out_ip: &mut [u8; 4]) -> Result<(), Error> {
    let mut buf = [0u8; 256];
    let bytes = name.as_bytes();
    let len = bytes.len().min(255);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
    // SAFETY: resolve syscall is safe here
    let r = unsafe { crate::syscall::syscall2(200, buf.as_ptr() as u64, out_ip.as_mut_ptr() as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// A network socket.
pub struct Socket {
    fd: i64,
}

impl Socket {
    /// Creates a new socket.
    pub fn new(domain: SocketDomain, stype: SocketType, protocol: i32) -> Result<Self, Error> {
        // SAFETY: socket syscall is safe here
        let r = unsafe { crate::syscall::syscall3(41, domain as u64, stype as u64, protocol as u64) };
        if r < 0 { Err(Error::from_i64(r)) } else { Ok(Socket { fd: r }) }
    }

    /// Binds the socket to a local address.
    pub fn bind(&self, addr: &[u8]) -> Result<(), Error> {
        // SAFETY: bind syscall is safe here
        let r = unsafe { crate::syscall::syscall3(49, self.fd as u64, addr.as_ptr() as u64, addr.len() as u64) };
        if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
    }

    /// Listens for incoming connections.
    pub fn listen(&self, backlog: i32) -> Result<(), Error> {
        // SAFETY: listen syscall is safe here
        let r = unsafe { crate::syscall::syscall2(50, self.fd as u64, backlog as u64) };
        if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
    }

    /// Accepts a new connection on the socket.
    pub fn accept(&self, addr: &mut [u8], addrlen: &mut u32) -> Result<Socket, Error> {
        // SAFETY: accept syscall is safe here
        let r = unsafe { crate::syscall::syscall3(43, self.fd as u64, addr.as_mut_ptr() as u64, addrlen as *mut u32 as u64) };
        if r < 0 { Err(Error::from_i64(r)) } else { Ok(Socket { fd: r }) }
    }

    /// Connects the socket to a remote address.
    pub fn connect(&self, addr: &[u8]) -> Result<(), Error> {
        // SAFETY: connect syscall is safe here
        let r = unsafe { crate::syscall::syscall3(42, self.fd as u64, addr.as_ptr() as u64, addr.len() as u64) };
        if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
    }

    /// Reads from the socket.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        io::read(self.fd, buf)
    }

    /// Writes to the socket.
    pub fn write(&self, buf: &[u8]) -> Result<usize, Error> {
        io::write(self.fd, buf)
    }

    /// Returns the underlying file descriptor.
    pub fn as_raw_fd(&self) -> i64 {
        self.fd
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        let _ = io::close(self.fd);
    }
}

/// Convenience functions for sockets.
pub fn socket(domain: u64, stype: u64, protocol: i32) -> Result<i64, Error> {
    let r = unsafe { crate::syscall::syscall3(41, domain, stype, protocol as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r) }
}

/// Binds a raw file descriptor to an address.
pub fn bind(fd: i64, addr: &[u8]) -> Result<(), Error> {
    let r = unsafe { crate::syscall::syscall3(49, fd as u64, addr.as_ptr() as u64, addr.len() as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Listens on a raw file descriptor.
pub fn listen(fd: i64, backlog: i32) -> Result<(), Error> {
    let r = unsafe { crate::syscall::syscall2(50, fd as u64, backlog as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Accepts a connection on a raw file descriptor.
pub fn accept(fd: i64, addr: &mut [u8], addrlen: &mut u32) -> Result<i64, Error> {
    let r = unsafe { crate::syscall::syscall3(43, fd as u64, addr.as_mut_ptr() as u64, addrlen as *mut u32 as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(r) }
}

/// Connects a raw file descriptor to an address.
pub fn connect(fd: i64, addr: &[u8]) -> Result<(), Error> {
    let r = unsafe { crate::syscall::syscall3(42, fd as u64, addr.as_ptr() as u64, addr.len() as u64) };
    if r < 0 { Err(Error::from_i64(r)) } else { Ok(()) }
}

/// Sends data through a raw socket file descriptor.
pub fn send(fd: i64, buf: &[u8]) -> Result<usize, Error> {
    io::write(fd, buf)
}

/// Receives data from a raw socket file descriptor.
pub fn recv(fd: i64, buf: &mut [u8]) -> Result<usize, Error> {
    io::read(fd, buf)
}

/// Legacy constants.
pub const AF_INET: u64 = 2;
/// Legacy constants.
pub const SOCK_STREAM: u64 = 1;

/// Basic HTTP/1.1 client.
pub struct HttpClient;

impl HttpClient {
    /// Fetches a URL via HTTP GET.
    pub fn get(_url: &str) -> Result<Vec<u8>, Error> {
        Err(Error::ENOSYS)
    }
}

/// Parses an IPv4 address from a string.
pub fn parse_ipv4(ip: &str) -> Option<u32> {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 { return None; }
    let mut addr = 0u32;
    for (i, part) in parts.iter().enumerate() {
        let val: u8 = part.parse().ok()?;
        addr |= (val as u32) << (i * 8);
    }
    Some(addr)
}
