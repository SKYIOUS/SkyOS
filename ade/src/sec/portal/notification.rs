#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::{ApplicationId, ServiceRequest, ServiceResponse};

pub(crate) fn handle_request(desktop: &mut Desktop, app: ApplicationId, req: &ServiceRequest) -> ServiceResponse {
    match req.method {
        "notify" => {
            // args packed as title\0body\0urgency\0timeout in req.args
            crate::util::desktop_api::notification::notify(desktop, app, "Notification", "?", 0, 5000);
            ServiceResponse { request_id: req.request_id, success: true, data: alloc::vec::Vec::new() }
        }
        "dismiss_all" => {
            crate::util::desktop_api::notification::dismiss_all(desktop, app);
            ServiceResponse { request_id: req.request_id, success: true, data: alloc::vec::Vec::new() }
        }
        _ => ServiceResponse { request_id: req.request_id, success: false, data: alloc::vec::Vec::new() },
    }
}
