#![allow(dead_code)]

use alloc::vec::Vec;
use crate::ipc::channel::{Channel, ChannelType};
use crate::ipc::message::{ApplicationId, ChannelId, Message};

/// IPC API v1.0 — STABLE
pub(crate) struct IpcServer {
    pub channels: Vec<Channel>,
    pub next_channel_id: u64,
    pub next_message_id: u64,
    pub pending_messages: Vec<Message>,
}

impl IpcServer {
    pub fn new() -> Self {
        IpcServer {
            channels: Vec::new(),
            next_channel_id: 0,
            next_message_id: 0,
            pending_messages: Vec::new(),
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

    pub fn tick(&mut self) {}
}
