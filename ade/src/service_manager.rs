//! Service framework — central service registry and lifecycle.
#![allow(dead_code)]

use crate::config::Config;
use crate::ipc::MessageBus;
use crate::perms::PermissionManager;

pub(crate) struct ServiceManager {
    pub config: Config,
    pub bus: MessageBus,
    pub perms: PermissionManager,
}

impl ServiceManager {
    pub fn new() -> Self {
        ServiceManager {
            config: Config::new(),
            bus: MessageBus::new(),
            perms: PermissionManager::new(),
        }
    }

    pub fn tick(&mut self) {
        let msgs = self.bus.drain();
        for msg in msgs {
            match msg {
                crate::ipc::IpcMessage::Request(req) => {
                    self.dispatch_request(req);
                }
                crate::ipc::IpcMessage::Response(resp) => {
                    core::mem::drop(resp);
                }
                crate::ipc::IpcMessage::Broadcast(bc) => {
                    core::mem::drop(bc);
                }
            }
        }
    }

    fn dispatch_request(&mut self, req: crate::ipc::IpcRequest) {
        match req.target {
            crate::ipc::IpcTarget::Service("config") => {
                if req.method == "get" && req.args.len() >= 2 {
                    let val = self.config.get(&req.args[0], &req.args[1]);
                    self.bus.respond(
                        req.seq,
                        val.is_some(),
                        val.map(|v| alloc::vec![alloc::string::String::from(v)])
                            .unwrap_or_default(),
                    );
                }
            }
            crate::ipc::IpcTarget::Service("perms") => {
                if req.method == "check" && req.args.len() >= 2 {
                    let pid: u64 = req.args[0].parse().unwrap_or(0);
                    let perm: u32 = req.args[1].parse().unwrap_or(0);
                    self.bus.respond(
                        req.seq,
                        true,
                        alloc::vec![alloc::string::String::from(
                            if self.perms.check(pid, perm) {
                                "granted"
                            } else {
                                "denied"
                            }
                        )],
                    );
                }
            }
            _ => {
                self.bus.respond(req.seq, false, alloc::vec![]);
            }
        }
    }

    pub fn shutdown(&mut self) {
        self.config = Config::new();
        self.perms = PermissionManager::new();
    }
}
