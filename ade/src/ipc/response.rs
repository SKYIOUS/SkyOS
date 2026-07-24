#![allow(dead_code)]

use alloc::vec::Vec;
use crate::ipc::message::RequestId;

pub(crate) struct ServiceResponse {
    pub request_id: RequestId,
    pub success: bool,
    pub data: Vec<u8>,
}
