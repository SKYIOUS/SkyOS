use crate::ipc::request::ServiceRequest;
use crate::ipc::response::ServiceResponse;
use alloc::vec::Vec;

/// Queues service requests/responses between the socket transport and the
/// security portal: requests arrive via `ipc_transport.ingest()`, are gated
/// and dispatched by `Desktop::process_ipc()`, and responses flow back
/// through `ipc_transport.deliver()`.
pub(crate) struct IpcServer {
    pub pending_requests: Vec<ServiceRequest>,
    pub pending_responses: Vec<ServiceResponse>,
}

impl IpcServer {
    pub fn new() -> Self {
        IpcServer {
            pending_requests: Vec::new(),
            pending_responses: Vec::new(),
        }
    }

    /// Queues a service request for the security portal.
    pub fn submit_request(&mut self, req: ServiceRequest) {
        self.pending_requests.push(req);
    }

    /// Drains queued service requests for dispatch this frame.
    pub fn drain_requests(&mut self) -> Vec<ServiceRequest> {
        core::mem::take(&mut self.pending_requests)
    }

    /// Queues a service response for the caller to collect.
    pub fn submit_response(&mut self, resp: ServiceResponse) {
        self.pending_responses.push(resp);
    }

    /// Drains completed service responses.
    pub fn drain_responses(&mut self) -> Vec<ServiceResponse> {
        core::mem::take(&mut self.pending_responses)
    }
}
