use crate::ipc::message::{ApplicationId, RequestId};
use crate::ipc::registry::ServiceId;
use alloc::string::String;
use alloc::vec::Vec;

pub(crate) struct ServiceRequest {
    pub request_id: RequestId,
    pub service: ServiceId,
    pub method: String,
    pub args: Vec<u8>,
    pub sender: ApplicationId,
}
