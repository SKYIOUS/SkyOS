use crate::core::desktop::Desktop;
use crate::ipc::{ApplicationId, ServiceRequest, ServiceResponse};

pub(crate) fn handle_request(
    desktop: &mut Desktop,
    app: ApplicationId,
    req: &ServiceRequest,
) -> ServiceResponse {
    match req.method.as_str() {
        "copy" => {
            crate::util::desktop_api::clipboard::copy(desktop, app, &req.args);
            ServiceResponse {
                request_id: req.request_id,
                success: true,
                data: alloc::vec::Vec::new(),
                recipient: app,
            }
        }
        "paste" => match crate::util::desktop_api::clipboard::paste(desktop, app) {
            Some(data) => ServiceResponse {
                request_id: req.request_id,
                success: true,
                data,
                recipient: app,
            },
            None => ServiceResponse {
                request_id: req.request_id,
                success: false,
                data: alloc::vec::Vec::new(),
                recipient: app,
            },
        },
        _ => ServiceResponse {
            request_id: req.request_id,
            success: false,
            data: alloc::vec::Vec::new(),
            recipient: app,
        },
    }
}
