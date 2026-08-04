#![allow(dead_code)]

use crate::util::package::database::{InstalledPackage, PackageDatabase, PackageMetadata};
use alloc::string::String;

pub(crate) struct PackageInstaller;

impl PackageInstaller {
    pub fn install(
        db: &mut PackageDatabase,
        metadata: PackageMetadata,
        install_path: &str,
    ) -> bool {
        if db.is_installed(&metadata.id) {
            return false;
        }
        db.installed.push(InstalledPackage {
            metadata: metadata.clone(),
            install_path: String::from(install_path),
            install_date_seconds: 0,
            enabled: true,
        });
        db.available.push(metadata);
        true
    }
}
