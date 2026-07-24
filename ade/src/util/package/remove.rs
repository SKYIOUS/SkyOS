#![allow(dead_code)]

use crate::util::package::database::PackageDatabase;

pub(crate) struct PackageRemover;

impl PackageRemover {
    pub fn remove(db: &mut PackageDatabase, package_id: &str) -> bool {
        let pos = match db.installed.iter().position(|p| p.metadata.id == package_id) {
            Some(p) => p,
            None => return false,
        };
        db.installed.remove(pos);
        true
    }
}
