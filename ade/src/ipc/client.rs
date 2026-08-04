#![allow(dead_code)]

use crate::ipc::message::{ApplicationId, ChannelId};
use alloc::vec::Vec;

pub(crate) struct IpcClient {
    pub app_id: ApplicationId,
    pub subscribed_channels: Vec<ChannelId>,
}

impl IpcClient {
    pub fn new(app_id: ApplicationId) -> Self {
        IpcClient {
            app_id,
            subscribed_channels: Vec::new(),
        }
    }

    pub fn subscribe(&mut self, channel_id: ChannelId) {
        if !self.subscribed_channels.contains(&channel_id) {
            self.subscribed_channels.push(channel_id);
        }
    }

    pub fn unsubscribe(&mut self, channel_id: ChannelId) {
        self.subscribed_channels.retain(|&c| c != channel_id);
    }
}
