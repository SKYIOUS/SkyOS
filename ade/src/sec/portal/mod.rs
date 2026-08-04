pub(crate) mod clipboard;
pub(crate) mod file_dialog;
pub(crate) mod notification;
pub(crate) mod settings;
pub(crate) mod window;

use crate::core::desktop::Desktop;
use crate::ipc::{ApplicationId, ServiceRequest, ServiceResponse};

pub(crate) fn dispatch(
    desktop: &mut Desktop,
    app: ApplicationId,
    req: &ServiceRequest,
) -> ServiceResponse {
    match req.service {
        crate::ipc::ServiceId::Clipboard => clipboard::handle_request(desktop, app, req),
        crate::ipc::ServiceId::Notification => notification::handle_request(desktop, app, req),
        crate::ipc::ServiceId::Settings => settings::handle_request(desktop, app, req),
        crate::ipc::ServiceId::Window => window::handle_request(desktop, app, req),
        crate::ipc::ServiceId::FileDialog => file_dialog::handle_request(desktop, app, req),
        _ => ServiceResponse {
            request_id: req.request_id,
            success: false,
            data: alloc::vec::Vec::new(),
            recipient: app,
        },
    }
}
