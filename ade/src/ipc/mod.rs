// IPC API v1.0 — STABLE
pub(crate) mod message;
pub(crate) mod permission;
pub(crate) mod registry;
pub(crate) mod request;
pub(crate) mod response;
pub(crate) mod server;
pub(crate) mod transport;

pub(crate) use message::ApplicationId;
pub(crate) use registry::{ServiceId, ServiceRegistry};
pub(crate) use request::ServiceRequest;
pub(crate) use response::ServiceResponse;
pub(crate) use server::IpcServer;
