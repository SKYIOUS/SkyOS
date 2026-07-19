//! Permissions layer — application capability grants.
#![allow(dead_code)]

use alloc::vec::Vec;

pub(crate) const PERM_CLIPBOARD: u32 = 0x0001;
pub(crate) const PERM_NOTIFICATIONS: u32 = 0x0002;
pub(crate) const PERM_FILESYSTEM: u32 = 0x0004;
pub(crate) const PERM_SETTINGS: u32 = 0x0008;
pub(crate) const PERM_NETWORK: u32 = 0x0010;
pub(crate) const PERM_EXEC: u32 = 0x0020;

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

    pub fn has(&self, perm: u32) -> bool {
        self.perms & perm == perm
    }
}

pub(crate) struct PermissionManager {
    pub app_perms: Vec<(u64, PermissionSet)>, // pid → permissions
}

impl PermissionManager {
    pub fn new() -> Self {
        PermissionManager {
            app_perms: Vec::new(),
        }
    }

    pub fn register(&mut self, pid: u64, perms: PermissionSet) {
        self.app_perms.push((pid, perms));
    }

    pub fn check(&self, pid: u64, perm: u32) -> bool {
        self.app_perms
            .iter()
            .find(|(p, _)| *p == pid)
            .map(|(_, set)| set.has(perm))
            .unwrap_or(false)
    }

    pub fn unregister(&mut self, pid: u64) {
        self.app_perms.retain(|(p, _)| *p != pid);
    }
}
