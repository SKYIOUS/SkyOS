#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone)]
pub(crate) struct PackageUpdate {
    pub package_id: String,
    pub current_version: String,
    pub new_version: String,
    pub release_date_seconds: u64,
    pub changelog: String,
}

pub(crate) struct PackageUpdater {
    pub updates: Vec<PackageUpdate>,
}

impl PackageUpdater {
    pub fn new() -> Self {
        PackageUpdater {
            updates: Vec::new(),
        }
    }

    pub fn check(&mut self, _db: &crate::util::package::database::PackageDatabase) {
        self.updates.clear();
        // placeholder — no networking
    }

    pub fn available_updates(&self) -> &[PackageUpdate] {
        &self.updates
    }

    pub fn get_update(&self, package_id: &str) -> Option<&PackageUpdate> {
        self.updates.iter().find(|u| u.package_id == package_id)
    }
}
