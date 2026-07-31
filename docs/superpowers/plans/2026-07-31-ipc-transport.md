# ADE IPC Transport Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give ADE a real, permission-gated IPC transport over AF_UNIX socketpairs so external app processes can call the security portal, making the wired security layer execute in production.

**Architecture:** A `libsarga::ipc` module defines the canonical wire protocol (framing + request/response codec + service-id bytes). ADE adds an `IpcTransport` that registers one socketpair fd per spawned app, polls it each tick, feeds decoded requests into the existing `IpcServer`/`process_ipc` gate+portal pipeline, and delivers responses back over the fd. A seed `ipc_echo` app proves the path end-to-end. One kernel bug is fixed: `sys_poll` on `UnixSocket` fds must report `POLLIN` only when the peer has queued data.

**Tech Stack:** Rust no_std (ade, libsarga, kernel crates), AF_UNIX socketpair (syscall 53), poll (7), read/write (0/1). No new dependencies.

## Global Constraints

- Build every step with: `$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"; cargo +nightly build --target x86_64-sarga.json --release -p <crate>`
- Kernel crate builds with: `cargo +nightly build` in `kernel/kernel`.
- `#![no_std]` everywhere; `extern crate alloc;` at crate roots.
- `// SAFETY:` required on every `unsafe` block; avoid unwrap/expect.
- No new dependencies beyond libsarga / existing workspace crates.
- Kernel source resolves under `C:\Users\nanda\Desktop\Github\SKYIOUS KERNEL\kernel\src\...` (junction into this repo's `kernel/kernel`).
- ADF wire messages are single-write/single-read: one `send` = one queue entry = one `recv` pops it whole. Never split a frame across writes.
- The request wire format carries NO sender: identity comes from the server's fd→pid mapping.
- Tests are compile-checked only (no host runner); they run at boot via `ade --selftest`.

---

### Task 1: Kernel poll correctness for UnixSocket fds

**Files:**
- Modify: `kernel/kernel/src/net/unix.rs` (add `socket_has_data` near `getpeercred_unix`)
- Modify: `kernel/kernel/src/syscalls/mod.rs:5333-5336`

**Interfaces:**
- Produces: `crate::net::unix::socket_has_data(handle: u64) -> bool` — true when the peer socket has queued data OR is closed.

- [ ] **Step 1: Add `socket_has_data` to net/unix.rs**

After the `getpeercred_unix` function (line 389), add:

```rust
pub fn socket_has_data(handle: u64) -> bool {
    let socks = UNIX_SOCKETS.lock();
    match socks.get(&handle) {
        Some(sock) => {
            let inner = sock.inner.lock();
            !inner.recv_queue.is_empty() || inner.closed
        }
        None => false,
    }
}
```

- [ ] **Step 2: Fix the sys_poll UnixSocket arm**

In `kernel/kernel/src/syscalls/mod.rs`, replace lines 5333-5336:

```rust
                Some(FileDescriptor::UnixSocket(_, _)) => {
                    if *events & POLLIN != 0 { *revents |= POLLIN; }
                    if *events & POLLOUT != 0 { *revents |= POLLOUT; }
                }
```

with:

```rust
                Some(FileDescriptor::UnixSocket(handle, _)) => {
                    if *events & POLLIN != 0 && crate::net::unix::socket_has_data(*handle) {
                        *revents |= POLLIN;
                    }
                    if *events & POLLOUT != 0 { *revents |= POLLOUT; }
                }
```

- [ ] **Step 3: Build the kernel**

Run (in repo root): `cargo +nightly build --manifest-path kernel/kernel/Cargo.toml`
Expected: `Finished` (net feature gates this file but it compiles both ways).

- [ ] **Step 4: Commit**

```bash
git add kernel/kernel/src/net/unix.rs kernel/kernel/src/syscalls/mod.rs
git commit -m "kernel: report POLLIN on unix sockets only when data queued"
```

---

### Task 2: libsarga::ipc — wire protocol module

**Files:**
- Create: `libsarga/src/ipc.rs`
- Modify: `libsarga/src/lib.rs` (add `pub mod ipc;` after line 17 `pub mod io;`)
- Modify: `libsarga/src/net.rs` (add `Unix = 1` variant to `SocketDomain` enum)

**Interfaces:**
- Produces:
  - `pub const MAX_IPC_MSG: usize = 4096;`
  - `pub const HEADER_LEN: usize = 4;`
  - `pub const SVC_CLIPBOARD: u8 = 0;` … `pub const SVC_POWER: u8 = 8;` (Clipboard, Notification, Launcher, FileDialog, Settings, Session, Window, Theme, Power — in enum order)
  - `pub fn write_frame(fd: i64, payload: &[u8]) -> Result<(), Error>`
  - `pub fn read_frame(fd: i64, buf: &mut [u8; MAX_IPC_MSG]) -> Result<usize, Error>` (0 = EOF)
  - `pub fn encode_request(req_id: u64, service: u8, method: &[u8], args: &[u8]) -> Vec<u8>`
  - `pub fn decode_request(buf: &[u8]) -> Option<(u64, u8, Vec<u8>, Vec<u8>)>`
  - `pub fn encode_response(req_id: u64, success: bool, data: &[u8]) -> Vec<u8>`
  - `pub fn decode_response(buf: &[u8]) -> Option<(u64, bool, Vec<u8>)>`

- [ ] **Step 1: Add `Unix = 1` to SocketDomain**

In `libsarga/src/net.rs`, in the `SocketDomain` enum, add as first variant:

```rust
    /// Unix domain sockets (socketpair).
    Unix = 1,
```

- [ ] **Step 2: Create `libsarga/src/ipc.rs`**

```rust
//! Inter-process communication wire protocol — canonical for ADE service calls.

use crate::errno::Error;
use alloc::vec::Vec;

/// Maximum payload size for one IPC message.
pub const MAX_IPC_MSG: usize = 4096;
/// Length-prefix header size (u32 LE).
pub const HEADER_LEN: usize = 4;

/// Canonical wire ids for ADE services (order must match ade ServiceId).
pub const SVC_CLIPBOARD: u8 = 0;
pub const SVC_NOTIFICATION: u8 = 1;
pub const SVC_LAUNCHER: u8 = 2;
pub const SVC_FILE_DIALOG: u8 = 3;
pub const SVC_SETTINGS: u8 = 4;
pub const SVC_SESSION: u8 = 5;
pub const SVC_WINDOW: u8 = 6;
pub const SVC_THEME: u8 = 7;
pub const SVC_POWER: u8 = 8;

/// Writes one complete frame (u32 LE length + payload) in a single write.
/// A single write becomes a single queued datagram at the peer; oversized
/// payloads are rejected rather than split.
pub fn write_frame(fd: i64, payload: &[u8]) -> Result<(), Error> {
    if payload.len() > MAX_IPC_MSG {
        return Err(Error::EINVAL);
    }
    let mut frame = alloc::vec![0u8; HEADER_LEN + payload.len()];
    frame[..HEADER_LEN].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    frame[HEADER_LEN..].copy_from_slice(payload);
    let n = crate::io::write(fd, &frame)?;
    if n != frame.len() {
        return Err(Error::EIO);
    }
    Ok(())
}

/// Reads one complete frame (header + payload) in a single read, returning the
/// payload length. Returns 0 on EOF (peer closed).
pub fn read_frame(fd: i64, buf: &mut [u8; MAX_IPC_MSG]) -> Result<usize, Error> {
    let mut frame = [0u8; HEADER_LEN + MAX_IPC_MSG];
    let n = crate::io::read(fd, &mut frame)?;
    if n == 0 {
        return Ok(0);
    }
    if n < HEADER_LEN {
        return Err(Error::EIO);
    }
    let len = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
    if len > MAX_IPC_MSG {
        return Err(Error::EINVAL);
    }
    if n != HEADER_LEN + len {
        return Err(Error::EIO);
    }
    buf[..len].copy_from_slice(&frame[HEADER_LEN..HEADER_LEN + len]);
    Ok(len)
}

/// request := u64 LE request_id | u8 service | u32 LE method_len | method | u32 LE args_len | args
pub fn encode_request(req_id: u64, service: u8, method: &[u8], args: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&req_id.to_le_bytes());
    out.push(service);
    out.extend_from_slice(&(method.len() as u32).to_le_bytes());
    out.extend_from_slice(method);
    out.extend_from_slice(&(args.len() as u32).to_le_bytes());
    out.extend_from_slice(args);
    out
}

pub fn decode_request(buf: &[u8]) -> Option<(u64, u8, Vec<u8>, Vec<u8>)> {
    if buf.len() < 13 {
        return None;
    }
    let req_id = u64::from_le_bytes(buf[0..8].try_into().ok()?);
    let service = buf[8];
    let mut pos = 9;
    let method_len = u32::from_le_bytes(buf[pos..pos + 4].try_into().ok()?) as usize;
    pos += 4;
    if pos + method_len + 4 > buf.len() {
        return None;
    }
    let method = buf[pos..pos + method_len].to_vec();
    pos += method_len;
    let args_len = u32::from_le_bytes(buf[pos..pos + 4].try_into().ok()?) as usize;
    pos += 4;
    if pos + args_len != buf.len() {
        return None;
    }
    let args = buf[pos..pos + args_len].to_vec();
    Some((req_id, service, method, args))
}

/// response := u64 LE request_id | u8 success | u32 LE data_len | data
pub fn encode_response(req_id: u64, success: bool, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&req_id.to_le_bytes());
    out.push(success as u8);
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(data);
    out
}

pub fn decode_response(buf: &[u8]) -> Option<(u64, bool, Vec<u8>)> {
    if buf.len() < 13 {
        return None;
    }
    let req_id = u64::from_le_bytes(buf[0..8].try_into().ok()?);
    let success = buf[8] != 0;
    let data_len = u32::from_le_bytes(buf[9..13].try_into().ok()?) as usize;
    if 13 + data_len != buf.len() {
        return None;
    }
    Some((req_id, success, buf[13..13 + data_len].to_vec()))
}
```

- [ ] **Step 3: Register the module**

In `libsarga/src/lib.rs`, after `pub mod io;` (line 17), add:

```rust
pub mod ipc;
```

- [ ] **Step 4: Build libsarga**

Run: `cargo +nightly build --target x86_64-sarga.json --release -p libsarga`
Expected: `Finished` with only pre-existing warnings.

- [ ] **Step 5: Commit**

```bash
git add libsarga/src/ipc.rs libsarga/src/lib.rs libsarga/src/net.rs
git commit -m "libsarga: add IPC wire protocol module (framing + service codec)"
```

---

### Task 3: ade type changes for wire-compatible requests

**Files:**
- Modify: `ade/src/ipc/request.rs:10` (`method: &'static str` → `String`)
- Modify: `ade/src/ipc/response.rs` (add `recipient: ApplicationId`)
- Modify: `ade/src/ipc/registry.rs` (add `to_wire`/`from_wire` to `ServiceId`)
- Modify: `ade/src/sec/portal/clipboard.rs`, `notification.rs`, `settings.rs`, `window.rs`, `file_dialog.rs`, `mod.rs` (add `recipient: app`, `match req.method.as_str()`, `_app` → `app`)
- Modify: `ade/src/util/testing/ipc.rs` (`method: "copy"` → `method: alloc::string::String::from("copy")`)
- Modify: `ade/src/core/desktop.rs` (denied path in `process_ipc`: add `recipient: app`)

**Interfaces:**
- Produces:
  - `ServiceRequest { request_id: RequestId, service: ServiceId, method: String, args: Vec<u8>, sender: ApplicationId }`
  - `ServiceResponse { request_id: RequestId, success: bool, data: Vec<u8>, recipient: ApplicationId }`
  - `ServiceId::to_wire(self) -> u8` and `ServiceId::from_wire(u8) -> Option<ServiceId>`

- [ ] **Step 1: Change ServiceRequest.method to String**

`ade/src/ipc/request.rs` — delete the `#![allow(dead_code)]` line, change the struct:

```rust
use alloc::string::String;
use alloc::vec::Vec;
use crate::ipc::message::{ApplicationId, RequestId};
use crate::ipc::registry::ServiceId;

pub(crate) struct ServiceRequest {
    pub request_id: RequestId,
    pub service: ServiceId,
    pub method: String,
    pub args: Vec<u8>,
    pub sender: ApplicationId,
}
```

- [ ] **Step 2: Add recipient to ServiceResponse**

`ade/src/ipc/response.rs`:

```rust
use alloc::vec::Vec;
use crate::ipc::message::{ApplicationId, RequestId};

pub(crate) struct ServiceResponse {
    pub request_id: RequestId,
    pub success: bool,
    pub data: Vec<u8>,
    /// Server-internal routing: the app this response is addressed to.
    /// Never serialized onto the wire.
    pub recipient: ApplicationId,
}
```

- [ ] **Step 3: Add ServiceId wire mapping**

In `ade/src/ipc/registry.rs`, add an impl block after the enum:

```rust
impl ServiceId {
    pub(crate) fn to_wire(self) -> u8 {
        match self {
            ServiceId::Clipboard => crate::libsarga::ipc::SVC_CLIPBOARD,
            ServiceId::Notification => crate::libsarga::ipc::SVC_NOTIFICATION,
            ServiceId::Launcher => crate::libsarga::ipc::SVC_LAUNCHER,
            ServiceId::FileDialog => crate::libsarga::ipc::SVC_FILE_DIALOG,
            ServiceId::Settings => crate::libsarga::ipc::SVC_SETTINGS,
            ServiceId::Session => crate::libsarga::ipc::SVC_SESSION,
            ServiceId::Window => crate::libsarga::ipc::SVC_WINDOW,
            ServiceId::Theme => crate::libsarga::ipc::SVC_THEME,
            ServiceId::Power => crate::libsarga::ipc::SVC_POWER,
        }
    }

    pub(crate) fn from_wire(w: u8) -> Option<ServiceId> {
        match w {
            crate::libsarga::ipc::SVC_CLIPBOARD => Some(ServiceId::Clipboard),
            crate::libsarga::ipc::SVC_NOTIFICATION => Some(ServiceId::Notification),
            crate::libsarga::ipc::SVC_LAUNCHER => Some(ServiceId::Launcher),
            crate::libsarga::ipc::SVC_FILE_DIALOG => Some(ServiceId::FileDialog),
            crate::libsarga::ipc::SVC_SETTINGS => Some(ServiceId::Settings),
            crate::libsarga::ipc::SVC_SESSION => Some(ServiceId::Session),
            crate::libsarga::ipc::SVC_WINDOW => Some(ServiceId::Window),
            crate::libsarga::ipc::SVC_THEME => Some(ServiceId::Theme),
            crate::libsarga::ipc::SVC_POWER => Some(ServiceId::Power),
            _ => None,
        }
    }
}
```

(Note: `crate::libsarga` resolves the dependency; if it fails, use `libsarga::ipc::...` directly — the ade crate depends on libsarga.)

- [ ] **Step 4: Update portal handlers**

In each of `clipboard.rs`, `notification.rs`, `settings.rs`, `window.rs`, `file_dialog.rs`:
- change `match req.method {` to `match req.method.as_str() {`
- change `_app` parameters to `app`
- add `recipient: app,` to every `ServiceResponse { ... }` literal

`file_dialog.rs` becomes:

```rust
pub(crate) fn handle_request(_desktop: &mut Desktop, app: ApplicationId, req: &ServiceRequest) -> ServiceResponse {
    // Placeholder — file dialog not yet wired to a UI
    ServiceResponse { request_id: req.request_id, success: false, data: alloc::vec::Vec::new(), recipient: app }
}
```

In `ade/src/sec/portal/mod.rs`, add `recipient: app` to the catch-all arm.

- [ ] **Step 5: Update the process_ipc denied path**

In `ade/src/core/desktop.rs` `process_ipc`, the denied response literal gains `recipient: app`:

```rust
            let resp = if allowed {
                crate::sec::portal::dispatch(self, app, &req)
            } else {
                crate::ipc::ServiceResponse {
                    request_id: req.request_id,
                    success: false,
                    data: alloc::vec::Vec::new(),
                    recipient: app,
                }
            };
```

- [ ] **Step 6: Update existing tests**

In `ade/src/util/testing/ipc.rs`, the three `ServiceRequest` literals change `method: "copy"` / `"paste"` to `method: alloc::string::String::from("copy")` / `String::from("paste")`.

- [ ] **Step 7: Build ade**

Run: `cargo +nightly build --target x86_64-sarga.json --release -p ade`
Expected: `Finished` with only pre-existing warnings (79).

- [ ] **Step 8: Commit**

```bash
git add ade/src/ipc/request.rs ade/src/ipc/response.rs ade/src/ipc/registry.rs ade/src/sec/portal ade/src/core/desktop.rs ade/src/util/testing/ipc.rs
git commit -m "ade: wire-compatible request/response types + ServiceId codec mapping"
```

---

### Task 4: IpcTransport + Desktop/launcher wiring

**Files:**
- Create: `ade/src/ipc/transport.rs`
- Modify: `ade/src/ipc/mod.rs` (add `pub(crate) mod transport;`)
- Modify: `ade/src/core/desktop.rs` (field, `tick`, `reap_children`)
- Modify: `ade/src/core/launcher.rs` (socketpair + `--ipc-fd` argv)

**Interfaces:**
- Produces:
  - `IpcTransport::new()`, `register(&mut self, pid: u64, fd: i64)`, `unregister(&mut self, pid: u64)`, `fd_for(&self, pid: u64) -> Option<i64>`
  - `IpcTransport::ingest(&mut self) -> Vec<ServiceRequest>` — polls all peer fds (POLLIN, timeout 0), reads one frame per ready fd, decodes with `sender = pid`; drops dead peers.
  - `IpcTransport::deliver(&mut self, responses: Vec<ServiceResponse>)` — writes each response frame to the recipient's fd; drops dead peers on error.

- [ ] **Step 1: Create `ade/src/ipc/transport.rs`**

```rust
use alloc::vec::Vec;
use crate::ipc::message::{ApplicationId, RequestId};
use crate::ipc::registry::ServiceId;
use crate::ipc::request::ServiceRequest;
use crate::ipc::response::ServiceResponse;
use libsarga::net::{PollFd, POLLIN};
use libsarga::ipc::{MAX_IPC_MSG, read_frame, write_frame};

struct IpcConnection {
    pid: u64,
    fd: i64,
}

/// Server-side transport over AF_UNIX socketpairs. One fd per spawned app.
/// The fd->pid mapping is the authoritative identity for permission checks —
/// the wire request never carries a sender.
pub(crate) struct IpcTransport {
    peers: Vec<IpcConnection>,
}

impl IpcTransport {
    pub(crate) fn new() -> Self {
        IpcTransport { peers: Vec::new() }
    }

    pub(crate) fn register(&mut self, pid: u64, fd: i64) {
        if self.fd_for(pid).is_none() {
            self.peers.push(IpcConnection { pid, fd });
        }
    }

    pub(crate) fn unregister(&mut self, pid: u64) {
        let mut i = 0;
        while i < self.peers.len() {
            if self.peers[i].pid == pid {
                let fd = self.peers.remove(i).fd;
                let _ = libsarga::io::close(fd);
            } else {
                i += 1;
            }
        }
    }

    pub(crate) fn fd_for(&self, pid: u64) -> Option<i64> {
        self.peers.iter().find(|c| c.pid == pid).map(|c| c.fd)
    }

    /// Reads one request frame per ready peer. Never blocks: poll(timeout 0)
    /// guarantees data is present before any read.
    pub(crate) fn ingest(&mut self) -> Vec<ServiceRequest> {
        let mut out = Vec::new();
        if self.peers.is_empty() {
            return out;
        }
        let mut pollfds: Vec<PollFd> = self
            .peers
            .iter()
            .map(|c| PollFd { fd: c.fd, events: POLLIN, revents: 0 })
            .collect();
        let ready = match libsarga::net::poll(&mut pollfds, 0) {
            Ok(n) => n,
            Err(_) => return out,
        };
        if ready <= 0 {
            return out;
        }
        let mut dead = Vec::new();
        for (i, pfd) in pollfds.iter().enumerate() {
            if pfd.revents & POLLIN == 0 {
                if pfd.revents != 0 {
                    dead.push(self.peers[i].pid);
                }
                continue;
            }
            let mut buf = [0u8; MAX_IPC_MSG];
            match read_frame(self.peers[i].fd, &mut buf) {
                Ok(0) => dead.push(self.peers[i].pid),
                Ok(n) => {
                    match libsarga::ipc::decode_request(&buf[..n]) {
                        Some((req_id, service, method, args)) => {
                            match (ServiceId::from_wire(service), alloc::string::String::from_utf8(method)) {
                                (Some(svc), Ok(m)) => out.push(ServiceRequest {
                                    request_id: RequestId(req_id),
                                    service: svc,
                                    method: m,
                                    args,
                                    sender: ApplicationId(self.peers[i].pid),
                                }),
                                _ => dead.push(self.peers[i].pid),
                            }
                        }
                        None => dead.push(self.peers[i].pid),
                    }
                }
                Err(_) => dead.push(self.peers[i].pid),
            }
        }
        for pid in dead {
            self.unregister(pid);
        }
        out
    }

    /// Writes each response frame to its recipient's fd. Responses for apps
    /// with no registered peer are dropped.
    pub(crate) fn deliver(&mut self, responses: Vec<ServiceResponse>) {
        let mut dead = Vec::new();
        for resp in responses {
            let pid = resp.recipient.0;
            if let Some(fd) = self.fd_for(pid) {
                let frame = libsarga::ipc::encode_response(
                    resp.request_id.0,
                    resp.success,
                    &resp.data,
                );
                if write_frame(fd, &frame).is_err() {
                    dead.push(pid);
                }
            }
        }
        for pid in dead {
            self.unregister(pid);
        }
    }
}
```

- [ ] **Step 2: Register the module**

In `ade/src/ipc/mod.rs`, add `pub(crate) mod transport;` after `pub(crate) mod server;`.

- [ ] **Step 3: Desktop field + wiring**

In `ade/src/core/desktop.rs`:
1. Add a field near `ipc_server`: `pub(crate) ipc_transport: crate::ipc::transport::IpcTransport,`
2. In `Desktop::new`, add `ipc_transport: crate::ipc::transport::IpcTransport::new(),`
3. In `tick()`, replace lines 269-270:

```rust
        self.reap_children();
        self.process_ipc();
```

with:

```rust
        self.reap_children();
        let reqs = self.ipc_transport.ingest();
        for req in reqs {
            self.ipc_server.submit_request(req);
        }
        self.process_ipc();
        let responses = self.ipc_server.drain_responses();
        self.ipc_transport.deliver(responses);
```

4. In `reap_children`, after `self.permissions.unregister(pid);` add `self.ipc_transport.unregister(pid);`

- [ ] **Step 4: Launcher socketpair wiring**

In `ade/src/core/launcher.rs`, replace lines 93-117 (the `if !path.is_empty()` block) with:

```rust
    if !path.is_empty() {
        let ipc_pair = libsarga::net::socketpair(
            libsarga::net::SocketDomain::Unix as u64,
            libsarga::net::SocketType::Stream as u64,
            0,
        )
        .ok();
        match libsarga::process::fork() {
            Ok(0) => {
                match ipc_pair {
                    Some((server_fd, client_fd)) => {
                        let _ = libsarga::io::close(server_fd);
                        let fd_arg = alloc::format!("{}", client_fd);
                        let argv = [path, "--ipc-fd", fd_arg.as_str()];
                        let _ = libsarga::process::execve(path, &argv, &[]);
                    }
                    None => {
                        let _ = libsarga::process::execve(path, &[path], &[]);
                    }
                }
                libsarga::process::exit(1);
            }
            Ok(pid) => {
                app_win.pid = Some(pid);
                let app_idx = desktop
                    .app_reg
                    .find_by_exec(path)
                    .map(|id| id.0)
                    .unwrap_or(0);
                desktop.lifecycle.register(pid, app_idx);
                desktop.permissions.register(pid, crate::sec::perms::default_grant());
                desktop.lifecycle.mark_running(pid);
                if let Some((server_fd, client_fd)) = ipc_pair {
                    let _ = libsarga::io::close(client_fd);
                    desktop.ipc_transport.register(pid, server_fd);
                }
                app_win
                    .content
                    .push(alloc::format!("[launched {} pid={}]", title, pid));
            }
            Err(e) => {
                if let Some((server_fd, client_fd)) = ipc_pair {
                    let _ = libsarga::io::close(server_fd);
                    let _ = libsarga::io::close(client_fd);
                }
                app_win.content.push(alloc::format!("[fork failed: {}]", e));
            }
        }
    }
```

- [ ] **Step 5: Build ade**

Run: `cargo +nightly build --target x86_64-sarga.json --release -p ade`
Expected: `Finished` (may emit one `field is never read` warning for `IpcTransport.peers` until Task 5 wires tests — acceptable pre-existing-style; the build is clean otherwise).

- [ ] **Step 6: Commit**

```bash
git add ade/src/ipc/transport.rs ade/src/ipc/mod.rs ade/src/core/desktop.rs ade/src/core/launcher.rs
git commit -m "ade: wire AF_UNIX socketpair IPC transport through spawn and tick"
```

---

### Task 5: Selftest coverage for the transport

**Files:**
- Modify: `ade/src/util/testing/ipc.rs` (add 5 tests)
- Modify: `ade/src/util/testing/mod.rs` (wire into `run_all`)

**Interfaces:**
- Consumes: `IpcTransport`, `libsarga::ipc::{encode_request, encode_response, decode_request, decode_response, read_frame, write_frame, SVC_*}`, `libsarga::net::{socketpair, PollFd, POLLIN, SocketDomain, SocketType}`.

- [ ] **Step 1: Add tests**

Append to `ade/src/util/testing/ipc.rs`:

```rust
pub(crate) fn test_service_wire() -> bool {
    use crate::ipc::registry::ServiceId;
    for s in [
        ServiceId::Clipboard,
        ServiceId::Notification,
        ServiceId::Launcher,
        ServiceId::FileDialog,
        ServiceId::Settings,
        ServiceId::Session,
        ServiceId::Window,
        ServiceId::Theme,
        ServiceId::Power,
    ] {
        if ServiceId::from_wire(s.to_wire()) != Some(s) {
            io::print_str("[test] FAIL test_service_wire: roundtrip failed\n");
            return false;
        }
    }
    if ServiceId::from_wire(99).is_some() {
        io::print_str("[test] FAIL test_service_wire: bogus wire id accepted\n");
        return false;
    }
    io::print_str("[test] PASS test_service_wire\n");
    true
}

pub(crate) fn test_codec_roundtrip() -> bool {
    let req = libsarga::ipc::encode_request(7, libsarga::ipc::SVC_NOTIFICATION, b"notify", b"t\0b\01\0");
    match libsarga::ipc::decode_request(&req) {
        Some((rid, svc, method, args)) => {
            if rid != 7
                || svc != libsarga::ipc::SVC_NOTIFICATION
                || method.as_slice() != b"notify"
                || args.as_slice() != b"t\0b\01\0"
            {
                io::print_str("[test] FAIL test_codec_roundtrip: request mismatch\n");
                return false;
            }
        }
        None => {
            io::print_str("[test] FAIL test_codec_roundtrip: request decode failed\n");
            return false;
        }
    }
    let resp = libsarga::ipc::encode_response(7, true, b"ok");
    match libsarga::ipc::decode_response(&resp) {
        Some((rid, ok, data)) => {
            if rid != 7 || !ok || data.as_slice() != b"ok" {
                io::print_str("[test] FAIL test_codec_roundtrip: response mismatch\n");
                return false;
            }
        }
        None => {
            io::print_str("[test] FAIL test_codec_roundtrip: response decode failed\n");
            return false;
        }
    }
    io::print_str("[test] PASS test_codec_roundtrip\n");
    true
}

pub(crate) fn test_frame_roundtrip() -> bool {
    let (a, b) = match libsarga::net::socketpair(
        libsarga::net::SocketDomain::Unix as u64,
        libsarga::net::SocketType::Stream as u64,
        0,
    ) {
        Ok(p) => p,
        Err(e) => {
            io::print_str(&alloc::format!("[test] FAIL test_frame_roundtrip: socketpair: {}\n", e));
            return false;
        }
    };
    let payload: Vec<u8> = (0..2000).map(|i| (i % 251) as u8).collect();
    if libsarga::ipc::write_frame(a, &payload).is_err() {
        io::print_str("[test] FAIL test_frame_roundtrip: write_frame failed\n");
        return false;
    }
    let mut buf = [0u8; libsarga::ipc::MAX_IPC_MSG];
    match libsarga::ipc::read_frame(b, &mut buf) {
        Ok(n) if n == payload.len() && buf[..n] == payload[..] => {}
        _ => {
            io::print_str("[test] FAIL test_frame_roundtrip: payload mismatch\n");
            return false;
        }
    }
    if libsarga::ipc::write_frame(a, &[0u8; libsarga::ipc::MAX_IPC_MSG + 1]).is_ok() {
        io::print_str("[test] FAIL test_frame_roundtrip: oversized write accepted\n");
        return false;
    }
    let _ = libsarga::io::close(a);
    let _ = libsarga::io::close(b);
    io::print_str("[test] PASS test_frame_roundtrip\n");
    true
}

pub(crate) fn test_poll_empty_socket() -> bool {
    let (a, b) = match libsarga::net::socketpair(
        libsarga::net::SocketDomain::Unix as u64,
        libsarga::net::SocketType::Stream as u64,
        0,
    ) {
        Ok(p) => p,
        Err(e) => {
            io::print_str(&alloc::format!("[test] FAIL test_poll_empty_socket: socketpair: {}\n", e));
            return false;
        }
    };
    let mut pfd = [PollFd { fd: a, events: POLLIN, revents: 0 }];
    match libsarga::net::poll(&mut pfd, 0) {
        Ok(n) if n == 0 && pfd[0].revents & POLLIN == 0 => {}
        _ => {
            io::print_str("[test] FAIL test_poll_empty_socket: empty socket reported ready (kernel poll bug)\n");
            return false;
        }
    }
    let _ = libsarga::io::close(a);
    let _ = libsarga::io::close(b);
    io::print_str("[test] PASS test_poll_empty_socket\n");
    true
}

pub(crate) fn test_transport_end_to_end(desktop: &mut crate::core::desktop::Desktop) -> bool {
    let (server_fd, client_fd) = match libsarga::net::socketpair(
        libsarga::net::SocketDomain::Unix as u64,
        libsarga::net::SocketType::Stream as u64,
        0,
    ) {
        Ok(p) => p,
        Err(e) => {
            io::print_str(&alloc::format!("[test] FAIL test_transport_end_to_end: socketpair: {}\n", e));
            return false;
        }
    };
    let pid = 60003u64;
    desktop.ipc_transport.register(pid, server_fd);
    desktop.permissions.register(pid, default_grant());

    // Client sends a clipboard "copy" request over the real socket.
    let req = libsarga::ipc::encode_request(9, libsarga::ipc::SVC_CLIPBOARD, b"copy", b"via transport");
    if libsarga::ipc::write_frame(client_fd, &req).is_err() {
        io::print_str("[test] FAIL test_transport_end_to_end: client write failed\n");
        return false;
    }

    // Server side: ingest -> gate+portal -> deliver.
    let reqs = desktop.ipc_transport.ingest();
    if reqs.len() != 1 || reqs[0].sender != ApplicationId(pid) || reqs[0].method != "copy" {
        io::print_str("[test] FAIL test_transport_end_to_end: ingest decode wrong\n");
        return false;
    }
    for r in reqs {
        desktop.ipc_server.submit_request(r);
    }
    desktop.process_ipc();
    let responses = desktop.ipc_server.drain_responses();
    desktop.ipc_transport.deliver(responses);

    // Client reads the response.
    let mut buf = [0u8; libsarga::ipc::MAX_IPC_MSG];
    match libsarga::ipc::read_frame(client_fd, &mut buf) {
        Ok(n) => match libsarga::ipc::decode_response(&buf[..n]) {
            Some((rid, success, _)) if rid == 9 && success => {}
            _ => {
                io::print_str("[test] FAIL test_transport_end_to_end: bad response\n");
                return false;
            }
        },
        _ => {
            io::print_str("[test] FAIL test_transport_end_to_end: read response failed\n");
            return false;
        }
    }

    // Denied path: a pid with no permissions gets success=false.
    let denied_pid = 60004u64;
    let (s2, c2) = match libsarga::net::socketpair(
        libsarga::net::SocketDomain::Unix as u64,
        libsarga::net::SocketType::Stream as u64,
        0,
    ) {
        Ok(p) => p,
        Err(_) => {
            io::print_str("[test] FAIL test_transport_end_to_end: second socketpair\n");
            return false;
        }
    };
    desktop.ipc_transport.register(denied_pid, s2);
    let req2 = libsarga::ipc::encode_request(10, libsarga::ipc::SVC_SETTINGS, b"open", b"");
    if libsarga::ipc::write_frame(c2, &req2).is_err() {
        io::print_str("[test] FAIL test_transport_end_to_end: denied client write\n");
        return false;
    }
    let reqs2 = desktop.ipc_transport.ingest();
    if reqs2.len() != 1 || reqs2[0].sender != ApplicationId(denied_pid) {
        io::print_str("[test] FAIL test_transport_end_to_end: denied ingest wrong\n");
        return false;
    }
    for r in reqs2 {
        desktop.ipc_server.submit_request(r);
    }
    desktop.process_ipc();
    let responses2 = desktop.ipc_server.drain_responses();
    desktop.ipc_transport.deliver(responses2);
    let mut buf2 = [0u8; libsarga::ipc::MAX_IPC_MSG];
    match libsarga::ipc::read_frame(c2, &mut buf2) {
        Ok(n) => match libsarga::ipc::decode_response(&buf2[..n]) {
            Some((rid, success, _)) if rid == 10 && !success => {}
            _ => {
                io::print_str("[test] FAIL test_transport_end_to_end: denied not rejected\n");
                return false;
            }
        },
        _ => {
            io::print_str("[test] FAIL test_transport_end_to_end: denied response read\n");
            return false;
        }
    }

    // Cleanup.
    desktop.ipc_transport.unregister(pid);
    desktop.ipc_transport.unregister(denied_pid);
    desktop.permissions.unregister(pid);
    let _ = libsarga::io::close(client_fd);
    let _ = libsarga::io::close(c2);

    io::print_str("[test] PASS test_transport_end_to_end\n");
    true
}
```

Note: the `clipboard.rs` `use crate::ipc::{ApplicationId, ...}` import is already in the test file via `use crate::ipc::message::{...}` — `ApplicationId` is re-exported there. Ensure `ApplicationId` and `RequestId` are in scope (they are, via the existing `use crate::ipc::message::{...}` at the top).

- [ ] **Step 2: Wire into run_all**

In `ade/src/util/testing/mod.rs`, insert before `ok &= services::test_notifications(desktop);`:

```rust
    ok &= ipc::test_service_wire();
    ok &= ipc::test_codec_roundtrip();
    ok &= ipc::test_frame_roundtrip();
    ok &= ipc::test_poll_empty_socket();
    ok &= ipc::test_transport_end_to_end(desktop);
```

- [ ] **Step 3: Build ade**

Run: `cargo +nightly build --target x86_64-sarga.json --release -p ade`
Expected: `Finished`.

- [ ] **Step 4: Commit**

```bash
git add ade/src/util/testing/ipc.rs ade/src/util/testing/mod.rs
git commit -m "ade: selftest coverage for IPC codec, framing, poll, and transport"
```

---

### Task 6: `ipc_echo` seed client app

**Files:**
- Create: `ipc_echo/Cargo.toml`, `ipc_echo/src/main.rs`
- Modify: `Cargo.toml` (workspace members)
- Modify: `build_initrd.py` (binaries dict)
- Modify: `ade/src/util/app_db.rs` (APPS entry)

- [ ] **Step 1: Create the crate**

`ipc_echo/Cargo.toml`:

```toml
[package]
name = "ipc_echo"
version = "0.1.0"
edition = "2021"
license.workspace = true

[dependencies]
libsarga = { path = "../libsarga" }
```

`ipc_echo/src/main.rs`:

```rust
#![no_std]
#![no_main]
extern crate alloc;
use libsarga::{args, io, sarga_main};

fn user_main() -> i32 {
    let mut ipc_fd = -1i64;
    let argc = args::argc();
    let mut i = 0;
    while i < argc {
        if args::get(i as usize) == Some("--ipc-fd") {
            if let Some(v) = args::get((i + 1) as usize) {
                if let Ok(n) = v.parse::<i64>() {
                    ipc_fd = n;
                }
            }
        }
        i += 1;
    }
    if ipc_fd < 0 {
        io::print_str("[ipc_echo] no --ipc-fd arg\n");
        return 1;
    }

    // Request a notification via the ADE security portal.
    let args_payload: &[u8] = b"IPC Echo\0Hello from ipc_echo via AF_UNIX socketpair\01\0";
    let req = libsarga::ipc::encode_request(1, libsarga::ipc::SVC_NOTIFICATION, b"notify", args_payload);
    if libsarga::ipc::write_frame(ipc_fd, &req).is_err() {
        io::print_str("[ipc_echo] send failed\n");
        return 1;
    }

    let mut buf = [0u8; libsarga::ipc::MAX_IPC_MSG];
    match libsarga::ipc::read_frame(ipc_fd, &mut buf) {
        Ok(0) => {
            io::print_str("[ipc_echo] connection closed by server\n");
            1
        }
        Ok(n) => match libsarga::ipc::decode_response(&buf[..n]) {
            Some((req_id, success, data)) => {
                io::print_str(&alloc::format!(
                    "[ipc_echo] response req={} success={} data_len={}\n",
                    req_id,
                    success,
                    data.len()
                ));
                if success { 0 } else { 1 }
            }
            None => {
                io::print_str("[ipc_echo] bad response frame\n");
                1
            }
        },
        Err(e) => {
            io::print_str(&alloc::format!("[ipc_echo] read failed: {}\n", e));
            1
        }
    }
}

sarga_main!(user_main);
```

- [ ] **Step 2: Add to workspace**

In `Cargo.toml` members list, add `"ipc_echo",` (alphabetical, after `installer`/`skyd-update`).

- [ ] **Step 3: Add to initrd**

In `build_initrd.py` `binaries` dict, add:

```python
    'bin/ipc_echo':     'ipc_echo',
```

- [ ] **Step 4: Add to the start menu registry**

In `ade/src/util/app_db.rs` APPS array, add:

```rust
    AppEntry { name: "IPC Echo",         cat: AppCategory::Utilities,    exec: "/bin/ipc_echo",      desc: "Test the ADE IPC channel",        icon: '=' },
```

- [ ] **Step 5: Build the workspace**

Run: `cargo +nightly build --target x86_64-sarga.json --release -p ipc_echo`
Expected: `Finished`.

- [ ] **Step 6: Commit**

```bash
git add ipc_echo Cargo.toml build_initrd.py ade/src/util/app_db.rs
git commit -m "feat: ipc_echo seed app exercises the ADE IPC transport end-to-end"
```

---

### Task 7: Documentation

**Files:**
- Modify: `docs/ade/desktop-environment-architecture.md`

- [ ] **Step 1: Document the transport in the arch doc**

1. Add to the Trace 3 section, under the existing "Wiring:" note, a second paragraph:

> Transport: each externally spawned app gets an AF_UNIX socketpair (`libsarga::net::socketpair(1, 1, 0)`). The server end lives in `IpcTransport` (ade/src/ipc/transport.rs); the client end is inherited across fork+exec and passed to the child as `--ipc-fd <n>`. Each frame is `u32 LE length | payload`, `<= 4096` bytes; one write = one queued datagram = one read pops it whole. Requests never carry a sender — the server maps fd → pid (authoritative identity for the permission gate). The kernel reports `POLLIN` on unix sockets only when data is queued, so the server can poll(timeout 0) then read without blocking.

2. Add to Trace 4's "Grant" section: identity for permission checks on the transport path is fd→pid derived at spawn, not client-declared.

3. Add to the Summary section item 3: "(real transport: AF_UNIX socketpair, see Trace 3)".

- [ ] **Step 2: Commit**

```bash
git add docs/ade/desktop-environment-architecture.md
git commit -m "docs: document the ADE IPC transport and fd-derived identity"
```

---

## Self-Review Checklist

- Spec coverage: kernel poll fix (Task 1), libsarga wire protocol (Task 2), ade type changes (Task 3), transport + wiring (Task 4), tests (Task 5), seed app (Task 6), docs (Task 7). All design sections mapped.
- Type consistency: `ServiceRequest.method: String` consistent across transport/portal/tests; `ServiceResponse.recipient` in every construction; `to_wire/from_wire` signatures consistent.
- No placeholders: all code complete.
