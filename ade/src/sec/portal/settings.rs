#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::{ApplicationId, ServiceRequest, ServiceResponse};

pub(crate) fn handle_request(desktop: &mut Desktop, _app: ApplicationId, req: &ServiceRequest) -> ServiceResponse {
    match req.method {
        "open" => {
            desktop.settings_app.open = true;
            ServiceResponse { request_id: req.request_id, success: true, data: alloc::vec::Vec::new() }
        }
        "close" => {
            desktop.settings_app.open = false;
            ServiceResponse { request_id: req.request_id, success: true, data: alloc::vec::Vec::new() }
        }
        _ => ServiceResponse { request_id: req.request_id, success: false, data: alloc::vec::Vec::new() },
    }
}
