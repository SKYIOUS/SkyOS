use alloc::vec::Vec;

/// IPC API v1.0 — STABLE
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ServiceId {
    Clipboard,
    Notification,
    FileDialog,
    Settings,
    Window,
}

impl ServiceId {
    /// The permission a caller must hold to use this service.
    pub(crate) fn required_permission(self) -> crate::ipc::permission::AppPermission {
        use crate::ipc::permission::{
            PERM_CLIPBOARD, PERM_FILESYSTEM, PERM_NOTIFICATIONS, PERM_SETTINGS, PERM_WINDOW_CONTROL,
        };
        match self {
            ServiceId::Clipboard => PERM_CLIPBOARD,
            ServiceId::Notification => PERM_NOTIFICATIONS,
            ServiceId::FileDialog => PERM_FILESYSTEM,
            ServiceId::Settings => PERM_SETTINGS,
            ServiceId::Window => PERM_WINDOW_CONTROL,
        }
    }

    pub(crate) fn to_wire(self) -> u8 {
        match self {
            ServiceId::Clipboard => libsarga::ipc::SVC_CLIPBOARD,
            ServiceId::Notification => libsarga::ipc::SVC_NOTIFICATION,
            ServiceId::FileDialog => libsarga::ipc::SVC_FILE_DIALOG,
            ServiceId::Settings => libsarga::ipc::SVC_SETTINGS,
            ServiceId::Window => libsarga::ipc::SVC_WINDOW,
        }
    }

    pub(crate) fn from_wire(w: u8) -> Option<ServiceId> {
        match w {
            libsarga::ipc::SVC_CLIPBOARD => Some(ServiceId::Clipboard),
            libsarga::ipc::SVC_NOTIFICATION => Some(ServiceId::Notification),
            libsarga::ipc::SVC_FILE_DIALOG => Some(ServiceId::FileDialog),
            libsarga::ipc::SVC_SETTINGS => Some(ServiceId::Settings),
            libsarga::ipc::SVC_WINDOW => Some(ServiceId::Window),
            _ => None,
        }
    }
}

/// IPC API v1.0 — STABLE
#[derive(Clone, Debug)]
pub(crate) struct ServiceInfo {
    pub id: ServiceId,
    pub name: &'static str,
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

    pub fn register(&mut self, info: ServiceInfo) {
        if !self.services.iter().any(|s| s.id == info.id) {
            self.services.push(info);
        }
    }

    /// Registers the services actually backed by `sec::portal` handlers.
    pub fn register_defaults(&mut self) {
        for (id, name) in [
            (ServiceId::Clipboard, "clipboard"),
            (ServiceId::Notification, "notification"),
            (ServiceId::FileDialog, "file_dialog"),
            (ServiceId::Settings, "settings"),
            (ServiceId::Window, "window"),
        ] {
            self.register(ServiceInfo {
                id,
                name,
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
}
