#![allow(dead_code)]

pub(crate) const PERM_CLIPBOARD: u32 = 0x0001;
pub(crate) const PERM_NOTIFICATIONS: u32 = 0x0002;
pub(crate) const PERM_FILESYSTEM: u32 = 0x0004;
pub(crate) const PERM_WINDOW_CONTROL: u32 = 0x0008;
pub(crate) const PERM_SETTINGS: u32 = 0x0010;
pub(crate) const PERM_POWER: u32 = 0x0020;
pub(crate) const PERM_CAMERA: u32 = 0x0040;
pub(crate) const PERM_MICROPHONE: u32 = 0x0080;
pub(crate) const PERM_NETWORK: u32 = 0x0100;
pub(crate) const PERM_USB: u32 = 0x0200;
pub(crate) const PERM_BLUETOOTH: u32 = 0x0400;
pub(crate) const PERM_LOCATION: u32 = 0x0800;

pub(crate) struct PermissionSet {
    pub perms: u32,
}

impl PermissionSet {
    pub fn new() -> Self {
        PermissionSet { perms: 0 }
    }

    pub fn all() -> Self {
        PermissionSet { perms: u32::MAX }
    }

    pub fn grant(&mut self, perm: u32) {
        self.perms |= perm;
    }

    pub fn revoke(&mut self, perm: u32) {
        self.perms &= !perm;
    }

    pub fn check(&self, perm: u32) -> bool {
        self.perms & perm == perm
    }

    pub fn has_any(&self, perms: u32) -> bool {
        self.perms & perms != 0
    }
}
