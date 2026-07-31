use alloc::vec::Vec;
use crate::ipc::message::{ApplicationId, RequestId};

pub(crate) struct ServiceResponse {
    pub request_id: RequestId,
    pub success: bool,
    pub data: Vec<u8>,
    /// Server-internal routing: the app this response is addressed to.
    /// Never serialized onto the wire.
    pub recipient: ApplicationId,
}
