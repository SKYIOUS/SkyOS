//! Permissions layer — application capability grants.
// Permission API v1.0 — STABLE
#![allow(dead_code)]

use alloc::vec::Vec;
use crate::ipc::permission::AppPermission;

/// Permission API v1.0
pub(crate) struct PermissionManager {
    pub app_perms: Vec<(u64, AppPermission)>, // pid → permissions
}

impl PermissionManager {
    pub fn new() -> Self {
        PermissionManager {
            app_perms: Vec::new(),
        }
    }

    pub fn register(&mut self, pid: u64, perms: AppPermission) {
        self.app_perms.push((pid, perms));
    }

    pub fn check(&self, pid: u64, perm: AppPermission) -> bool {
        self.app_perms
            .iter()
            .find(|(p, _)| *p == pid)
            .map(|(_, set)| set.contains(perm))
            .unwrap_or(false)
    }

    pub fn granted(&self, pid: u64) -> Option<AppPermission> {
        self.app_perms
            .iter()
            .find(|(p, _)| *p == pid)
            .map(|(_, set)| *set)
    }

    pub fn unregister(&mut self, pid: u64) {
        self.app_perms.retain(|(p, _)| *p != pid);
    }
}

/// Default grant for launched apps: everyday capabilities, no power or
/// hardware access. // ponytail: flat grant until app manifests drive per-app
/// permissions.
pub(crate) fn default_grant() -> AppPermission {
    AppPermission::CLIPBOARD
        | AppPermission::NOTIFICATIONS
        | AppPermission::FILESYSTEM
        | AppPermission::WINDOW_CONTROL
        | AppPermission::SETTINGS
}
