#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone)]
pub(crate) struct PackageDependency {
    pub name: String,
    pub version_min: String,
    pub version_max: String,
    pub optional: bool,
}

#[derive(Clone)]
pub(crate) struct PackageMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub category: String,
    pub size_bytes: u64,
    pub dependencies: Vec<PackageDependency>,
    pub homepage: String,
    pub license: String,
}

#[derive(Clone)]
pub(crate) struct InstalledPackage {
    pub metadata: PackageMetadata,
    pub install_path: String,
    pub install_date_seconds: u64,
    pub enabled: bool,
}

pub(crate) struct PackageDatabase {
    pub installed: Vec<InstalledPackage>,
    pub available: Vec<PackageMetadata>,
}

impl PackageDatabase {
    pub fn new() -> Self {
        PackageDatabase {
            installed: Vec::new(),
            available: Vec::new(),
        }
    }

    pub fn is_installed(&self, id: &str) -> bool {
        self.installed.iter().any(|p| p.metadata.id == id)
    }

    pub fn get(&self, id: &str) -> Option<&InstalledPackage> {
        self.installed.iter().find(|p| p.metadata.id == id)
    }

    pub fn search(&self, query: &str) -> Vec<&PackageMetadata> {
        self.available
            .iter()
            .filter(|p| {
                p.name.contains(query) || p.description.contains(query)
            })
            .collect()
    }

    pub fn by_category(&self, category: &str) -> Vec<&PackageMetadata> {
        self.available.iter().filter(|p| p.category == category).collect()
    }

    pub fn check_dependencies(&self, metadata: &PackageMetadata) -> bool {
        for dep in &metadata.dependencies {
            if !dep.optional
                && !self.installed.iter().any(|p| p.metadata.name == dep.name && p.enabled)
            {
                return false;
            }
        }
        true
    }
}
