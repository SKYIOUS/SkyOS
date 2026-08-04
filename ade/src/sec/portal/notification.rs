#![allow(dead_code)]

use crate::core::desktop::Desktop;
use crate::ipc::{ApplicationId, ServiceRequest, ServiceResponse};

pub(crate) fn handle_request(
    desktop: &mut Desktop,
    app: ApplicationId,
    req: &ServiceRequest,
) -> ServiceResponse {
    match req.method.as_str() {
        "notify" => {
            // args packed as title\0body\0urgency\0timeout in req.args
            let parts: alloc::vec::Vec<&[u8]> = req.args.split(|&b| b == 0).collect();
            let title = core::str::from_utf8(parts.first().copied().unwrap_or(b"")).unwrap_or("");
            let body = core::str::from_utf8(parts.get(1).copied().unwrap_or(b"")).unwrap_or("");
            let urgency = parts
                .get(2)
                .and_then(|p| core::str::from_utf8(p).ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let timeout = parts
                .get(3)
                .and_then(|p| core::str::from_utf8(p).ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(5000);
            crate::util::desktop_api::notification::notify(
                desktop, app, title, body, urgency, timeout,
            );
            ServiceResponse {
                request_id: req.request_id,
                success: true,
                data: alloc::vec::Vec::new(),
                recipient: app,
            }
        }
        "dismiss_all" => {
            crate::util::desktop_api::notification::dismiss_all(desktop, app);
            ServiceResponse {
                request_id: req.request_id,
                success: true,
                data: alloc::vec::Vec::new(),
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
