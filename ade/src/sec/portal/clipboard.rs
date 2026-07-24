#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::{ApplicationId, ServiceRequest, ServiceResponse};

pub(crate) fn handle_request(desktop: &mut Desktop, app: ApplicationId, req: &ServiceRequest) -> ServiceResponse {
    match req.method {
        "copy" => {
            crate::util::desktop_api::clipboard::copy(desktop, app, &req.args);
            ServiceResponse { request_id: req.request_id, success: true, data: alloc::vec::Vec::new() }
        }
        "paste" => {
            match crate::util::desktop_api::clipboard::paste(desktop, app) {
                Some(text) => ServiceResponse {
                    request_id: req.request_id,
                    success: true,
                    data: text.as_bytes().to_vec(),
                },
                None => ServiceResponse { request_id: req.request_id, success: false, data: alloc::vec::Vec::new() },
            }
        }
        _ => ServiceResponse { request_id: req.request_id, success: false, data: alloc::vec::Vec::new() },
    }
}
