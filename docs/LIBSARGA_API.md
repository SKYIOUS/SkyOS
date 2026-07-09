# libsarga API Reference

The unified standard library for SARGA OS.

## Error Handling
All functions return `Result<T, Error>`.
```rust
use libsarga::errno::Error;
match libsarga::io::open("/etc/hostname", 0) {
    Ok(fd) => { /* ... */ },
    Err(e) => { println!("Error: {}", e); }
}
```

## Modules

### io
Low-level file and stream I/O.
- `open(path: &str, flags: i32) -> Result<i64, Error>`
- `read(fd: i64, buf: &mut [u8]) -> Result<usize, Error>`
- `write(fd: i64, buf: &[u8]) -> Result<usize, Error>`
- `close(fd: i64) -> Result<(), Error>`
- `select(nfds, readfds, writefds, exceptfds, timeout_ms) -> Result<i32, Error>`

### fs
High-level filesystem operations.
- `stat(path: &str) -> Result<Stat, Error>`
- `statfs(path: &str) -> Result<Statfs, Error>`
- `write_file(path: &str, content: &str) -> Result<(), Error>`
- `read_to_string(path: &str) -> Result<String, Error>`

### net
Networking and socket management.
- `poll(fds: &mut [PollFd], timeout_ms: i32) -> Result<i32, Error>`
- `resolve(name: &str, out_ip: &mut [u8; 4]) -> Result<(), Error>`
- `Socket::new(domain, stype, protocol) -> Result<Socket, Error>`
- `Socket::bind(&self, addr: &SockAddrIn) -> Result<(), Error>`
- `Socket::connect(&self, addr: &SockAddrIn) -> Result<(), Error>`

### gpu
Graphics Hardware Acceleration (DRM/GEM).
- `get_display_info() -> Result<DisplayInfo, Error>`
- `create_dumb(width, height, bpp) -> Result<DumbInfo, Error>`
- `map_dumb(id) -> Result<*mut u32, Error>`
- `flip() -> Result<(), Error>`

### process
Process control and management.
- `fork() -> Result<u64, Error>`
- `execve(path, args, env) -> Result<(), Error>`
- `wait(pid) -> Result<i32, Error>`
- `waitpid(pid, options) -> Result<(u64, i32), Error>`
- `kill(pid, sig) -> Result<(), Error>`

### sync
Synchronization primitives.
- `Mutex<T>`: A mutual exclusion primitive based on futex.
- `TlsKey`: Thread-Local Storage management.

### hash
Cryptographic operations.
- `pbkdf2_sha256(password, salt, out, iterations) -> Result<u32, Error>`
