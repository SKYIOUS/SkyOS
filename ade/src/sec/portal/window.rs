#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::{ApplicationId, ServiceRequest, ServiceResponse};

pub(crate) fn handle_request(
    desktop: &mut Desktop,
    app: ApplicationId,
    req: &ServiceRequest,
) -> ServiceResponse {
    match req.method.as_str() {
        "list" => {
            let mut data = alloc::vec::Vec::new();
            for w in desktop.wm.iter() {
                data.extend_from_slice(w.title.as_bytes());
                data.push(b'\n');
            }
            ServiceResponse {
                request_id: req.request_id,
                success: true,
                data,
                recipient: app,
            }
        }
        _ => ServiceResponse {
            request_id: req.request_id,
            success: false,
            data: alloc::vec::Vec::new(),
            recipient: app,
        },
    }
}
