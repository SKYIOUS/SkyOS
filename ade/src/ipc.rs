//! IPC foundation — messages, requests, responses, broadcast.
#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Debug)]
pub(crate) enum IpcMessage {
    Request(IpcRequest),
    Response(IpcResponse),
    Broadcast(IpcBroadcast),
}

#[derive(Clone, Debug)]
pub(crate) struct IpcRequest {
    pub seq: u64,
    pub target: IpcTarget,
    pub method: &'static str,
    pub args: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct IpcResponse {
    pub seq: u64,
    pub success: bool,
    pub data: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct IpcBroadcast {
    pub topic: &'static str,
    pub data: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum IpcTarget {
    Window(u64),
    Application(u64),
    Service(&'static str),
    Desktop,
}

pub(crate) struct MessageBus {
    pub seq: u64,
    pub pending: Vec<IpcMessage>,
}

impl MessageBus {
    pub fn new() -> Self {
        MessageBus {
            seq: 0,
            pending: Vec::new(),
        }
    }

    pub fn request(&mut self, target: IpcTarget, method: &'static str, args: Vec<String>) -> u64 {
        self.seq += 1;
        let seq = self.seq;
        self.pending.push(IpcMessage::Request(IpcRequest {
            seq,
            target,
            method,
            args,
        }));
        seq
    }

    pub fn respond(&mut self, seq: u64, success: bool, data: Vec<String>) {
        self.pending
            .push(IpcMessage::Response(IpcResponse { seq, success, data }));
    }

    pub fn broadcast(&mut self, topic: &'static str, data: Vec<String>) {
        self.pending
            .push(IpcMessage::Broadcast(IpcBroadcast { topic, data }));
    }

    pub fn drain(&mut self) -> Vec<IpcMessage> {
        core::mem::take(&mut self.pending)
    }
}
