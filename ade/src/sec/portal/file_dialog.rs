#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::{ApplicationId, ServiceRequest, ServiceResponse};

pub(crate) fn handle_request(_desktop: &mut Desktop, app: ApplicationId, req: &ServiceRequest) -> ServiceResponse {
    // Placeholder — file dialog not yet wired to a UI
    ServiceResponse { request_id: req.request_id, success: false, data: alloc::vec::Vec::new(), recipient: app }
}
