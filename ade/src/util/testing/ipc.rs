#![allow(dead_code)]

use crate::ipc::message::{
    ApplicationId, ChannelId, IpcTarget, Message, MessageBus, MessageId, MessagePayload, MessageType,
    RequestId,
};
use crate::ipc::channel::{Channel, ChannelType};
use crate::ipc::permission::AppPermission;
use crate::ipc::registry::{ServiceId, ServiceInfo, ServiceRegistry};
use crate::ipc::request::ServiceRequest;
use crate::sec::perms::{default_grant, PermissionManager};
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

pub(crate) fn test_permission_manager() -> bool {
    let mut pm = PermissionManager::new();
    if pm.granted(1).is_some() {
        io::print_str("[test] FAIL test_permission_manager: empty manager grants\n");
        return false;
    }
    pm.register(1, default_grant());
    if !pm.check(1, AppPermission::CLIPBOARD) {
        io::print_str("[test] FAIL test_permission_manager: CLIPBOARD not granted\n");
        return false;
    }
    if pm.check(1, AppPermission::POWER) {
        io::print_str("[test] FAIL test_permission_manager: POWER should be denied\n");
        return false;
    }
    if pm.check(2, AppPermission::CLIPBOARD) {
        io::print_str("[test] FAIL test_permission_manager: unregistered pid granted\n");
        return false;
    }
    pm.unregister(1);
    if pm.granted(1).is_some() {
        io::print_str("[test] FAIL test_permission_manager: unregister failed\n");
        return false;
    }
    io::print_str("[test] PASS test_permission_manager\n");
    true
}

pub(crate) fn test_register_defaults() -> bool {
    let mut reg = ServiceRegistry::new();
    reg.register_defaults();
    if reg.services.len() != 9 {
        io::print_str("[test] FAIL test_register_defaults: expected 9 services\n");
        return false;
    }
    if reg.find(ServiceId::Clipboard).map(|s| s.name) != Some("clipboard") {
        io::print_str("[test] FAIL test_register_defaults: clipboard missing\n");
        return false;
    }
    if !reg.all().iter().all(|s| s.available) {
        io::print_str("[test] FAIL test_register_defaults: not all available\n");
        return false;
    }
    io::print_str("[test] PASS test_register_defaults\n");
    true
}

pub(crate) fn test_exit_class() -> bool {
    use crate::sys::lifecycle::{exit_class, ExitClass};
    if exit_class(0) != ExitClass::Clean {
        io::print_str("[test] FAIL test_exit_class: clean exit misclassified\n");
        return false;
    }
    if exit_class(-1) != ExitClass::Killed {
        io::print_str("[test] FAIL test_exit_class: killed misclassified\n");
        return false;
    }
    if exit_class(1) != ExitClass::Error(1) {
        io::print_str("[test] FAIL test_exit_class: error exit misclassified\n");
        return false;
    }
    if exit_class(127) != ExitClass::Error(127) {
        io::print_str("[test] FAIL test_exit_class: error exit boundary misclassified\n");
        return false;
    }
    if exit_class(137) != ExitClass::Signal(9) {
        io::print_str("[test] FAIL test_exit_class: SIGKILL death misclassified\n");
        return false;
    }
    if exit_class(139) != ExitClass::Signal(11) {
        io::print_str("[test] FAIL test_exit_class: SIGSEGV death misclassified\n");
        return false;
    }
    io::print_str("[test] PASS test_exit_class\n");
    true
}

pub(crate) fn test_ipc_gate_granted(desktop: &mut crate::core::desktop::Desktop) -> bool {
    let app = ApplicationId(60001);
    desktop.permissions.register(app.0, default_grant());

    desktop.ipc_server.submit_request(ServiceRequest {
        request_id: RequestId(1),
        service: ServiceId::Clipboard,
        method: alloc::string::String::from("copy"),
        args: b"hello".to_vec(),
        sender: app,
    });
    desktop.process_ipc();
    let mut resp = desktop.ipc_server.drain_responses();
    if resp.len() != 1 {
        io::print_str("[test] FAIL test_ipc_gate_granted: copy no response\n");
        return false;
    }
    if !resp.remove(0).success {
        io::print_str("[test] FAIL test_ipc_gate_granted: copy denied\n");
        return false;
    }

    desktop.ipc_server.submit_request(ServiceRequest {
        request_id: RequestId(2),
        service: ServiceId::Clipboard,
        method: alloc::string::String::from("paste"),
        args: Vec::new(),
        sender: app,
    });
    desktop.process_ipc();
    let mut resp = desktop.ipc_server.drain_responses();
    if resp.len() != 1 {
        io::print_str("[test] FAIL test_ipc_gate_granted: paste no response\n");
        return false;
    }
    let r = resp.remove(0);
    if !r.success || r.data != b"hello".to_vec() {
        io::print_str("[test] FAIL test_ipc_gate_granted: paste wrong value\n");
        return false;
    }

    desktop.permissions.unregister(app.0);
    io::print_str("[test] PASS test_ipc_gate_granted\n");
    true
}

pub(crate) fn test_ipc_gate_denied(desktop: &mut crate::core::desktop::Desktop) -> bool {
    let app = ApplicationId(60002); // never granted any permission
    desktop.ipc_server.submit_request(ServiceRequest {
        request_id: RequestId(3),
        service: ServiceId::Clipboard,
        method: alloc::string::String::from("copy"),
        args: b"secret".to_vec(),
        sender: app,
    });
    desktop.process_ipc();
    let mut resp = desktop.ipc_server.drain_responses();
    if resp.len() != 1 {
        io::print_str("[test] FAIL test_ipc_gate_denied: no response\n");
        return false;
    }
    if resp.remove(0).success {
        io::print_str("[test] FAIL test_ipc_gate_denied: ungranted app succeeded\n");
        return false;
    }
    io::print_str("[test] PASS test_ipc_gate_denied\n");
    true
}
