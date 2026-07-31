#![allow(dead_code)]

use alloc::vec::Vec;

/// IPC API v1.0 — STABLE
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ServiceId {
    Clipboard,
    Notification,
    Launcher,
    FileDialog,
    Settings,
    Session,
    Window,
    Theme,
    Power,
}

impl ServiceId {
    pub(crate) fn to_wire(self) -> u8 {
        match self {
            ServiceId::Clipboard => libsarga::ipc::SVC_CLIPBOARD,
            ServiceId::Notification => libsarga::ipc::SVC_NOTIFICATION,
            ServiceId::Launcher => libsarga::ipc::SVC_LAUNCHER,
            ServiceId::FileDialog => libsarga::ipc::SVC_FILE_DIALOG,
            ServiceId::Settings => libsarga::ipc::SVC_SETTINGS,
            ServiceId::Session => libsarga::ipc::SVC_SESSION,
            ServiceId::Window => libsarga::ipc::SVC_WINDOW,
            ServiceId::Theme => libsarga::ipc::SVC_THEME,
            ServiceId::Power => libsarga::ipc::SVC_POWER,
        }
    }

    pub(crate) fn from_wire(w: u8) -> Option<ServiceId> {
        match w {
            libsarga::ipc::SVC_CLIPBOARD => Some(ServiceId::Clipboard),
            libsarga::ipc::SVC_NOTIFICATION => Some(ServiceId::Notification),
            libsarga::ipc::SVC_LAUNCHER => Some(ServiceId::Launcher),
            libsarga::ipc::SVC_FILE_DIALOG => Some(ServiceId::FileDialog),
            libsarga::ipc::SVC_SETTINGS => Some(ServiceId::Settings),
            libsarga::ipc::SVC_SESSION => Some(ServiceId::Session),
            libsarga::ipc::SVC_WINDOW => Some(ServiceId::Window),
            libsarga::ipc::SVC_THEME => Some(ServiceId::Theme),
            libsarga::ipc::SVC_POWER => Some(ServiceId::Power),
            _ => None,
        }
    }
}

/// IPC API v1.0 — STABLE
#[derive(Clone, Debug)]
pub(crate) struct ServiceInfo {
    pub id: ServiceId,
    pub name: &'static str,
    pub version: u32,
    pub required_permissions: u32,
    pub available: bool,
}

/// IPC API v1.0 — STABLE
pub(crate) struct ServiceRegistry {
    pub services: Vec<ServiceInfo>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        ServiceRegistry {
            services: Vec::new(),
        }
    }

    pub fn find(&self, id: ServiceId) -> Option<&ServiceInfo> {
        self.services.iter().find(|s| s.id == id)
    }

    pub fn find_by_name(&self, name: &str) -> Option<&ServiceInfo> {
        self.services.iter().find(|s| s.name == name)
    }

    pub fn find_by_permission(&self, perm: u32) -> Vec<&ServiceInfo> {
        self.services
            .iter()
            .filter(|s| s.required_permissions & perm == perm)
            .collect()
    }

    pub fn register(&mut self, info: ServiceInfo) {
        if !self.services.iter().any(|s| s.id == info.id) {
            self.services.push(info);
        }
    }

    /// Registers the services actually backed by `sec::portal` handlers, each
    /// gated by the same permission its `desktop_api` entry enforces.
    pub fn register_defaults(&mut self) {
        use crate::ipc::permission::{
            PERM_CLIPBOARD, PERM_FILESYSTEM, PERM_NOTIFICATIONS, PERM_POWER, PERM_SETTINGS,
            PERM_WINDOW_CONTROL,
        };
        for (id, name, required) in [
            (ServiceId::Clipboard, "clipboard", PERM_CLIPBOARD.bits()),
            (ServiceId::Notification, "notification", PERM_NOTIFICATIONS.bits()),
            (ServiceId::Launcher, "launcher", PERM_FILESYSTEM.bits()),
            (ServiceId::FileDialog, "file_dialog", PERM_FILESYSTEM.bits()),
            (ServiceId::Settings, "settings", PERM_SETTINGS.bits()),
            (ServiceId::Session, "session", PERM_POWER.bits()),
            (ServiceId::Window, "window", PERM_WINDOW_CONTROL.bits()),
            (ServiceId::Theme, "theme", PERM_SETTINGS.bits()),
            (ServiceId::Power, "power", PERM_POWER.bits()),
        ] {
            self.register(ServiceInfo {
                id,
                name,
                version: 1,
                required_permissions: required,
                available: true,
            });
        }
    }

    pub fn set_available(&mut self, id: ServiceId, available: bool) {
        for s in &mut self.services {
            if s.id == id {
                s.available = available;
                return;
            }
        }
    }

    pub fn all(&self) -> &[ServiceInfo] {
        &self.services
    }

    pub fn discover(&self, name: &str) -> Option<&ServiceInfo> {
        self.find_by_name(name)
    }

    pub fn discover_by_permission(&self, perm: u32) -> Vec<&ServiceInfo> {
        self.find_by_permission(perm)
    }
}
