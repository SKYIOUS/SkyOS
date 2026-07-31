# ADE IPC Transport — Design

**Date:** 2026-07-31
**Status:** Approved (user delegated design + autonomous implementation)
**Branch:** `feat/ade-ipc-transport`

## Problem

The ADE security architecture (portal dispatch, permission gate, service registry)
is fully wired but never executes in production. `IpcServer::submit_request` is
called only from the test harness; no external app process can send a service
request. The IPC layer is a set of in-memory queues with no transport.

Root cause: **there is no message transport between ADE and the apps it forks.**
`IpcClient`, `Channel`, `MessageBus` are test-only scaffold.

## Goal

Make the security layer live: any app process can send a service request over a
real kernel-backed channel; ADE gates it on permissions, dispatches it through
the portal, and delivers the response back to that process. Keep the existing
in-memory `IpcServer` core (used by tests directly) and add the transport as the
external interface feeding it.

## Transport primitive: AF_UNIX socketpair

Verified kernel facts (kernel/src/syscalls/mod.rs, kernel/src/net/unix.rs):

- `sys_socketpair(domain=1, type=1|2, 0)` creates two fds in `fd_table` whose
  peers share a `recv_queue` (`VecDeque<Vec<u8>>`). One `send` = one queued
  datagram; one `recv` pops the whole entry.
- `read`/`write` (syscalls 0/1) route to `recvfrom_unix`/`sendto_unix` — reads
  **block** when the queue is empty, writes never block.
- `execve` copies `fd_table` with **no CLOEXEC filtering** → the client fd
  survives `fork` + `execve`. Verified in mod.rs:4455-4477.
- Closing one end marks both ends `closed` and wakes the peer → the app's
  blocked read returns 0 (EOF) and the server's next write gets `EPIPE`.

**Kernel bug to fix:** `sys_poll` on a `UnixSocket` always reports `POLLIN` and
`POLLOUT` (mod.rs:5333-5336) without checking `recv_queue`. The ADE server
must not block, so it polls with timeout 0 and reads only ready fds; a lying
`POLLIN` would make a read block forever. Fix: report `POLLIN` only when the
peer socket has queued data or is closed. Add `net::unix::socket_has_data()`
and use it in the `sys_poll` arm.

## Wire protocol (libsarga::ipc — single source of truth)

`libsarga` is the only shared library between ADE (server) and the app crates
(clients). The protocol lives there so server and clients can never drift.

```
frame    := u32 LE payload_len | payload           (payload_len <= MAX_IPC_MSG)
request  := u64 LE request_id | u8 service | u32 LE method_len | method
          | u32 LE args_len | args
response := u64 LE request_id | u8 success | u32 LE data_len | data
```

- `MAX_IPC_MSG = 4096`. Messages are single-write/single-read: one `send` = one
  queue entry = one `recv` pops it whole. Partial reads are impossible by
  construction; oversized messages are rejected at encode time.
- **The request carries no sender.** The server derives identity from the fd →
  pid mapping it registered at spawn. A client cannot spoof another app's pid
  for permission checks.
- Service ids are canonical bytes (`SVC_CLIPBOARD=0 … SVC_POWER=8`) defined in
  libsarga::ipc; ade's `ServiceId` maps to/from them.

## Components

### Kernel (2 small changes)
1. `net/unix.rs`: `pub fn socket_has_data(handle: u64) -> bool` (queue non-empty
   or closed).
2. `syscalls/mod.rs`: `sys_poll` UnixSocket arm uses `socket_has_data` for
   `POLLIN`; `POLLOUT` stays always-ready (writes never block).

### libsarga::ipc (new module)
- `SocketDomain::Unix = 1` variant.
- Constants: `MAX_IPC_MSG`, `SVC_*`.
- Framing: `write_frame(fd, &[u8]) -> Result<(), Error>`,
  `read_frame(fd, &mut [u8; MAX]) -> Result<usize, Error>` (0 = EOF).
- Codec: `encode_request/decode_request`, `encode_response/decode_response`.

### ade ipc
- `ServiceRequest.method: &'static str` → `String` (decodable from wire).
- `ServiceResponse` gains `recipient: ApplicationId` (server-internal routing
  to the peer fd; not serialized).
- `ServiceId::to_wire()/from_wire()` via libsarga constants.
- `ipc/codec.rs`: type-aware encode/decode wrappers.
- `ipc/transport.rs`: `IpcTransport`
  - `peers: Vec<IpcConnection { pid: u64, fd: i64 }>`
  - `register(pid, fd)`, `unregister(pid)`, `fd_for(pid) -> Option<i64>`
  - `ingest() -> Vec<ServiceRequest>`: poll all peer fds (`POLLIN`, timeout 0),
    `read_frame` each ready fd, decode with `sender = pid`; drop dead fds
    (`POLLNVAL`, EOF, decode failure).
  - `deliver(&[ServiceResponse])`: `write_frame` to the recipient's fd; on
    `EPIPE`/EOF unregister the peer.

### Desktop wiring
- `Desktop` owns an `IpcTransport`.
- `tick()` order: `reap_children` → transport `ingest` (→ `submit_request`) →
  `process_ipc` (existing gate + portal; sets `resp.recipient`) →
  transport `deliver(drain_responses)`.
- `reap_children`: also `ipc_transport.unregister(pid)` (closes the fd).
- `launcher::spawn_app_at`: create socketpair **before** fork; child closes the
  server end and `execve`s with an extra `--ipc-fd <client_fd>` arg (fd survives
  exec); parent closes the client end and registers `(pid, server_fd)` after
  fork success. Uniform for all external spawns; apps that ignore `--ipc-fd`
  just never send. In-process apps (Settings/About/Explorer) are untouched.

### Seed client: `ipc_echo` app
- New crate, workspace member, `'bin/ipc_echo'` in `build_initrd.py`.
- Parses `--ipc-fd <n>`, sends a `Notification::notify` request, awaits the
  response, prints the result, exits. Proves the whole path end-to-end in a
  real separate process and gives future apps a reference client.

## Tests (selftest harness, in-process but real syscalls)

- Codec round-trip (request + response).
- `ServiceId::to_wire/from_wire` round-trip.
- Transport loopback: `socketpair` in-process; register a pid on one end; the
  other end acts as the client (write a framed request via libsarga::ipc);
  `ingest` decodes it with `sender = pid`; run gate + portal; `deliver`; the
  client end reads the response frame. Exercises real kernel socketpair + poll
  (with the fix) + framing + codec + permission gate + portal dispatch.
- Gate-denied: request for a service the pid lacks permission for →
  `success = false`.
- EOF/dead-peer handling: client closes its end; `ingest`/`deliver` drop the
  peer without blocking.

## Non-goals

- No app manifest / per-app permission customization (flat `default_grant`
  stays).
- No portal handlers for Launcher/Session/Theme/Power (out of scope).
- No bulk data / shared-memory transport (shm + futex noted for later).
- No response timeouts or request dedup (frames are synchronous and small).

## Performance

`poll` is one syscall for all peers per frame (timeout 0); reads and writes are
single `recv`/`send` on a per-socket queue — no copies beyond the queue push.
The 64-request/frame cap in `process_ipc` stays as the soft-real-time guard.

## Verification

- Kernel: build the kernel crate after the poll fix.
- libsarga, ade: build after each step.
- Selftest: compile-checked; runs at boot via `ade --selftest`.
- Manual: launch `ipc_echo` from a shell (or start menu) on the booted system.
