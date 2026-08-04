#![allow(dead_code)]

use crate::ipc::channel::{Channel, ChannelType};
use crate::ipc::message::{ApplicationId, ChannelId, Message};
use crate::ipc::request::ServiceRequest;
use crate::ipc::response::ServiceResponse;
use alloc::vec::Vec;

/// IPC API v1.0 — STABLE
pub(crate) struct IpcServer {
    pub channels: Vec<Channel>,
    pub next_channel_id: u64,
    pub next_message_id: u64,
    pub pending_messages: Vec<Message>,
    pub pending_requests: Vec<ServiceRequest>,
    pub pending_responses: Vec<ServiceResponse>,
}

impl IpcServer {
    pub fn new() -> Self {
        IpcServer {
            channels: Vec::new(),
            next_channel_id: 0,
            next_message_id: 0,
            pending_messages: Vec::new(),
            pending_requests: Vec::new(),
            pending_responses: Vec::new(),
        }
    }

    /// IPC API v1.0
    pub fn create_channel(&mut self, channel_type: ChannelType) -> ChannelId {
        let id = ChannelId(self.next_channel_id);
        self.next_channel_id += 1;
        self.channels.push(Channel::new(id, channel_type));
        id
    }

    /// IPC API v1.0
    pub fn subscribe(&mut self, channel_id: ChannelId, app: ApplicationId) -> bool {
        for ch in &mut self.channels {
            if ch.id == channel_id {
                ch.subscribe(app);
                return true;
            }
        }
        false
    }

    /// IPC API v1.0
    pub fn unsubscribe(&mut self, channel_id: ChannelId, app: ApplicationId) -> bool {
        for ch in &mut self.channels {
            if ch.id == channel_id {
                ch.unsubscribe(app);
                return true;
            }
        }
        false
    }

    /// IPC API v1.0
    pub fn send(&mut self, msg: Message) {
        self.pending_messages.push(msg);
    }

    /// IPC API v1.0
    pub fn drain_pending(&mut self) -> Vec<Message> {
        core::mem::take(&mut self.pending_messages)
    }

    /// Queues a service request for the security portal.
    pub fn submit_request(&mut self, req: ServiceRequest) {
        self.pending_requests.push(req);
    }

    /// Drains queued service requests for dispatch this frame.
    pub fn drain_requests(&mut self) -> Vec<ServiceRequest> {
        core::mem::take(&mut self.pending_requests)
    }

    /// Queues a service response for the caller to collect.
    pub fn submit_response(&mut self, resp: ServiceResponse) {
        self.pending_responses.push(resp);
    }

    /// Drains completed service responses.
    pub fn drain_responses(&mut self) -> Vec<ServiceResponse> {
        core::mem::take(&mut self.pending_responses)
    }

    pub fn tick(&mut self) {}
}
