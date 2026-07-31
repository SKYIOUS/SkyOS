#![allow(dead_code)]
use bitflags::bitflags;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) struct AppPermission: u32 {
        const CLIPBOARD = 0x0001;
        const NOTIFICATIONS = 0x0002;
        const FILESYSTEM = 0x0004;
        const WINDOW_CONTROL = 0x0008;
        const SETTINGS = 0x0010;
        const POWER = 0x0020;
        const CAMERA = 0x0040;
        const MICROPHONE = 0x0080;
        const NETWORK = 0x0100;
        const USB = 0x0200;
        const BLUETOOTH = 0x0400;
        const LOCATION = 0x0800;
    }
}

// Legacy aliases for compatibility
pub(crate) use AppPermission as PermissionSet;

impl PermissionSet {
    pub fn new() -> Self {
        Self::empty()
    }

    pub fn grant(&mut self, perm: Self) {
        self.insert(perm);
    }

    pub fn revoke(&mut self, perm: Self) {
        self.remove(perm);
    }

    pub fn check(&self, perm: Self) -> bool {
        self.contains(perm)
    }

    pub fn has_any(&self, perms: Self) -> bool {
        self.intersects(perms)
    }
}

pub(crate) const PERM_CLIPBOARD: AppPermission = AppPermission::CLIPBOARD;
pub(crate) const PERM_NOTIFICATIONS: AppPermission = AppPermission::NOTIFICATIONS;
pub(crate) const PERM_FILESYSTEM: AppPermission = AppPermission::FILESYSTEM;
pub(crate) const PERM_WINDOW_CONTROL: AppPermission = AppPermission::WINDOW_CONTROL;
pub(crate) const PERM_SETTINGS: AppPermission = AppPermission::SETTINGS;
pub(crate) const PERM_POWER: AppPermission = AppPermission::POWER;
pub(crate) const PERM_CAMERA: AppPermission = AppPermission::CAMERA;
pub(crate) const PERM_MICROPHONE: AppPermission = AppPermission::MICROPHONE;
pub(crate) const PERM_NETWORK: AppPermission = AppPermission::NETWORK;
pub(crate) const PERM_USB: AppPermission = AppPermission::USB;
pub(crate) const PERM_BLUETOOTH: AppPermission = AppPermission::BLUETOOTH;
pub(crate) const PERM_LOCATION: AppPermission = AppPermission::LOCATION;
