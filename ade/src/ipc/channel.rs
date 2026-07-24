#![allow(dead_code)]

use alloc::vec::Vec;
use crate::ipc::message::{ApplicationId, ChannelId, Message};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChannelType {
    RequestReply,
    Broadcast,
    Notification,
    Signal,
    OneToMany,
    ManyToOne,
}

pub(crate) struct Channel {
    pub id: ChannelId,
    pub channel_type: ChannelType,
    pub subscribers: Vec<ApplicationId>,
    pub messages: Vec<Message>,
}

impl Channel {
    pub fn new(id: ChannelId, channel_type: ChannelType) -> Self {
        Channel {
            id,
            channel_type,
            subscribers: Vec::new(),
            messages: Vec::new(),
        }
    }

    pub fn subscribe(&mut self, app: ApplicationId) {
        if !self.subscribers.contains(&app) {
            self.subscribers.push(app);
        }
    }

    pub fn unsubscribe(&mut self, app: ApplicationId) {
        self.subscribers.retain(|&a| a != app);
    }

    pub fn push(&mut self, msg: Message) {
        self.messages.push(msg);
    }

    pub fn drain(&mut self) -> Vec<Message> {
        core::mem::take(&mut self.messages)
    }
}
