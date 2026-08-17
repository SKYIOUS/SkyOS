//! IPC identity types shared by the socket request/response protocol.

/// IPC API v1.0 — STABLE
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RequestId(pub u64);

/// IPC API v1.0 — STABLE
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ApplicationId(pub u64);
