#![allow(dead_code)]

use crate::ipc::message::{
    ApplicationId, ChannelId, IpcTarget, Message, MessageBus, MessageId, MessagePayload, MessageType,
};
use crate::ipc::channel::{Channel, ChannelType};
use crate::ipc::registry::{ServiceId, ServiceInfo, ServiceRegistry};
use alloc::vec::Vec;
use libsarga::io;

pub(crate) fn test_message_bus() -> bool {
    let mut bus = MessageBus::new();
    if bus.seq != 0 {
        io::print_str("[test] FAIL test_message_bus: seq not 0\n");
        return false;
    }
    if !bus.pending.is_empty() {
        io::print_str("[test] FAIL test_message_bus: pending not empty\n");
        return false;
    }

    let seq = bus.request(IpcTarget::Desktop, "ping", Vec::new());
    if seq != 1 {
        io::print_str("[test] FAIL test_message_bus: seq not incremented\n");
        return false;
    }
    if bus.pending.len() != 1 {
        io::print_str("[test] FAIL test_message_bus: pending count != 1\n");
        return false;
    }

    let drained = bus.drain();
    if drained.len() != 1 {
        io::print_str("[test] FAIL test_message_bus: drain count wrong\n");
        return false;
    }
    if bus.pending.len() != 0 {
        io::print_str("[test] FAIL test_message_bus: drain did not clear pending\n");
        return false;
    }

    bus.respond(1, true, Vec::new());
    if bus.pending.len() != 1 {
        io::print_str("[test] FAIL test_message_bus: respond count wrong\n");
        return false;
    }
    bus.broadcast("test.topic", Vec::new());
    if bus.pending.len() != 2 {
        io::print_str("[test] FAIL test_message_bus: broadcast count wrong\n");
        return false;
    }

    io::print_str("[test] PASS test_message_bus\n");
    true
}

pub(crate) fn test_service_registry() -> bool {
    let mut reg = ServiceRegistry::new();
    if reg.services.len() != 0 {
        io::print_str("[test] FAIL test_service_registry: not empty\n");
        return false;
    }

    let info = ServiceInfo {
        id: ServiceId::Clipboard,
        name: "clipboard",
        version: 1,
        required_permissions: 0,
        available: true,
    };
    reg.register(info.clone());
    if reg.services.len() != 1 {
        io::print_str("[test] FAIL test_service_registry: register failed\n");
        return false;
    }
    reg.register(info);
    if reg.services.len() != 1 {
        io::print_str("[test] FAIL test_service_registry: duplicate register added\n");
        return false;
    }

    let found = reg.find(ServiceId::Clipboard);
    if found.is_none() {
        io::print_str("[test] FAIL test_service_registry: find by id failed\n");
        return false;
    }
    if found.unwrap().name != "clipboard" {
        io::print_str("[test] FAIL test_service_registry: name mismatch\n");
        return false;
    }

    let by_name = reg.find_by_name("clipboard");
    if by_name.is_none() {
        io::print_str("[test] FAIL test_service_registry: find_by_name failed\n");
        return false;
    }

    reg.set_available(ServiceId::Clipboard, false);
    let after = reg.find(ServiceId::Clipboard).unwrap();
    if after.available {
        io::print_str("[test] FAIL test_service_registry: set_available failed\n");
        return false;
    }

    if reg.all().len() != 1 {
        io::print_str("[test] FAIL test_service_registry: all() count wrong\n");
        return false;
    }

    io::print_str("[test] PASS test_service_registry\n");
    true
}

pub(crate) fn test_channels() -> bool {
    let id_a = ApplicationId(1);
    let ch_id = ChannelId(42);

    let mut ch = Channel::new(ch_id, ChannelType::Broadcast);
    if ch.id != ch_id {
        io::print_str("[test] FAIL test_channels: channel id mismatch\n");
        return false;
    }
    if !ch.subscribers.is_empty() {
        io::print_str("[test] FAIL test_channels: subscribers not empty\n");
        return false;
    }

    ch.subscribe(id_a);
    if ch.subscribers.len() != 1 {
        io::print_str("[test] FAIL test_channels: subscribe count wrong\n");
        return false;
    }
    ch.subscribe(id_a);
    if ch.subscribers.len() != 1 {
        io::print_str("[test] FAIL test_channels: duplicate subscribe added\n");
        return false;
    }

    ch.unsubscribe(id_a);
    if !ch.subscribers.is_empty() {
        io::print_str("[test] FAIL test_channels: unsubscribe failed\n");
        return false;
    }

    let msg = Message {
        id: MessageId(1),
        sender: id_a,
        receiver: ApplicationId(0),
        msg_type: MessageType::Broadcast,
        payload: MessagePayload::Text(b"hello".to_vec()),
        timestamp: 0,
        flags: 0,
    };
    ch.push(msg);
    if ch.messages.len() != 1 {
        io::print_str("[test] FAIL test_channels: push count wrong\n");
        return false;
    }

    let drained = ch.drain();
    if drained.len() != 1 {
        io::print_str("[test] FAIL test_channels: drain count wrong\n");
        return false;
    }
    if !ch.messages.is_empty() {
        io::print_str("[test] FAIL test_channels: drain did not clear\n");
        return false;
    }

    io::print_str("[test] PASS test_channels\n");
    true
}
