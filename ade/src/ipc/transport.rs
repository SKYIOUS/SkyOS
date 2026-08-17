use crate::ipc::message::{ApplicationId, RequestId};
use crate::ipc::registry::ServiceId;
use crate::ipc::request::ServiceRequest;
use crate::ipc::response::ServiceResponse;
use alloc::vec::Vec;
use libsarga::ipc::{read_frame, write_frame, MAX_IPC_MSG};
use libsarga::net::{PollFd, POLLIN};

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
            .map(|c| PollFd {
                fd: c.fd,
                events: POLLIN,
                revents: 0,
            })
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
                Ok(n) => match libsarga::ipc::decode_request(&buf[..n]) {
                    Some((req_id, service, method, args)) => {
                        match (
                            ServiceId::from_wire(service),
                            alloc::string::String::from_utf8(method),
                        ) {
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
                },
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
                let frame =
                    libsarga::ipc::encode_response(resp.request_id.0, resp.success, &resp.data);
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
