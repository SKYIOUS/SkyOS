//! Package Integration Layer — package database, metadata, update backend, dependencies.
//!
//! Manages installed packages, their metadata, dependencies, and updates.

use alloc::string::String;
use alloc::vec::Vec;

/// Package dependency
#[derive(Clone)]
pub struct PackageDependency {
    pub name: String,
    pub version_min: String,
    pub version_max: String,
    pub optional: bool,
}

/// Package metadata
#[derive(Clone)]
pub struct PackageMetadata {
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

/// Installed package entry
#[derive(Clone)]
pub struct InstalledPackage {
    pub metadata: PackageMetadata,
    pub install_path: String,
    pub install_date_seconds: u64,
    pub enabled: bool,
}

/// Package repository
#[derive(Clone)]
pub struct PackageRepository {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

/// Package update information
#[derive(Clone)]
pub struct PackageUpdate {
    pub package_id: String,
    pub current_version: String,
    pub new_version: String,
    pub release_date_seconds: u64,
    pub changelog: String,
}

/// Package Manager
pub struct PackageManager {
    /// Installed packages
    installed: Vec<InstalledPackage>,
    /// Available packages in repository
    available: Vec<PackageMetadata>,
    /// Configured repositories
    repositories: Vec<PackageRepository>,
    /// Pending updates
    updates: Vec<PackageUpdate>,
}

impl PackageManager {
    /// Create a new package manager
    pub fn new() -> Self {
        PackageManager {
            installed: Vec::new(),
            available: Vec::new(),
            repositories: Vec::new(),
            updates: Vec::new(),
        }
    }

    /// Register repository
    pub fn add_repository(&mut self, repo: PackageRepository) {
        if !self.repositories.iter().any(|r| r.name == repo.name) {
            self.repositories.push(repo);
        }
    }

    /// Remove repository
    pub fn remove_repository(&mut self, repo_name: &str) {
        self.repositories.retain(|r| r.name != repo_name);
    }

    /// Get repositories
    pub fn repositories(&self) -> &[PackageRepository] {
        &self.repositories
    }

    /// Install a package
    pub fn install_package(&mut self, metadata: PackageMetadata, install_path: &str) -> bool {
        if self.installed.iter().any(|p| p.metadata.id == metadata.id) {
            return false;
        }

        self.installed.push(InstalledPackage {
            metadata: metadata.clone(),
            install_path: String::from(install_path),
            install_date_seconds: 0,
            enabled: true,
        });

        self.available.push(metadata);
        true
    }

    /// Uninstall a package
    pub fn uninstall_package(&mut self, package_id: &str) -> bool {
        if let Some(pos) = self.installed.iter().position(|p| p.metadata.id == package_id) {
            self.installed.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get installed packages
    pub fn installed_packages(&self) -> &[InstalledPackage] {
        &self.installed
    }

    /// Get installed package by ID
    pub fn get_installed_package(&self, package_id: &str) -> Option<&InstalledPackage> {
        self.installed.iter().find(|p| p.metadata.id == package_id)
    }

    /// Enable/disable package
    pub fn set_package_enabled(&mut self, package_id: &str, enabled: bool) -> bool {
        if let Some(pkg) = self.installed.iter_mut().find(|p| p.metadata.id == package_id) {
            pkg.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Get available packages
    pub fn available_packages(&self) -> &[PackageMetadata] {
        &self.available
    }

    /// Search packages
    pub fn search_packages(&self, query: &str) -> Vec<&PackageMetadata> {
        self.available
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query.to_lowercase())
                    || p.description.to_lowercase().contains(&query.to_lowercase())
            })
            .collect()
    }

    /// Get packages by category
    pub fn packages_by_category(&self, category: &str) -> Vec<&PackageMetadata> {
        self.available
            .iter()
            .filter(|p| p.category == category)
            .collect()
    }

    /// Check package dependencies
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

    /// Update package index
    pub fn update_package_index(&mut self, packages: Vec<PackageMetadata>) {
        self.available.clear();
        self.available.extend(packages);
    }

    /// Check for updates
    pub fn check_for_updates(&mut self) {
        self.updates.clear();

        for installed in &self.installed {
            if let Some(available) = self.available.iter().find(|p| p.id == installed.metadata.id)
            {
                // Simple version comparison (assumes semantic versioning)
                if available.version != installed.metadata.version {
                    self.updates.push(PackageUpdate {
                        package_id: installed.metadata.id.clone(),
                        current_version: installed.metadata.version.clone(),
                        new_version: available.version.clone(),
                        release_date_seconds: 0,
                        changelog: String::from("See release notes"),
                    });
                }
            }
        }
    }

    /// Get available updates
    pub fn available_updates(&self) -> &[PackageUpdate] {
        &self.updates
    }

    /// Get update for package
    pub fn get_update(&self, package_id: &str) -> Option<&PackageUpdate> {
        self.updates.iter().find(|u| u.package_id == package_id)
    }

    /// Count installed packages
    pub fn installed_count(&self) -> u32 {
        self.installed.len() as u32
    }

    /// Count enabled packages
    pub fn enabled_count(&self) -> u32 {
        self.installed.iter().filter(|p| p.enabled).count() as u32
    }

    /// Get total installed size
    pub fn total_installed_size_bytes(&self) -> u64 {
        self.installed.iter().map(|p| p.metadata.size_bytes).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_creation() {
        let pm = PackageManager::new();
        assert_eq!(pm.installed_count(), 0);
    }

    #[test]
    fn test_install_package() {
        let mut pm = PackageManager::new();
        let meta = PackageMetadata {
            id: String::from("test-pkg"),
            name: String::from("Test Package"),
            version: String::from("1.0.0"),
            author: String::from("Test"),
            description: String::from("Test package"),
            category: String::from("Utility"),
            size_bytes: 1024,
            dependencies: Vec::new(),
            homepage: String::from(""),
            license: String::from("MIT"),
        };
        assert!(pm.install_package(meta, "/usr/bin/test"));
        assert_eq!(pm.installed_count(), 1);
    }

    #[test]
    fn test_search_packages() {
        let mut pm = PackageManager::new();
        let meta = PackageMetadata {
            id: String::from("example"),
            name: String::from("Example App"),
            version: String::from("1.0.0"),
            author: String::from("Test"),
            description: String::from("An example application"),
            category: String::from("Utility"),
            size_bytes: 1024,
            dependencies: Vec::new(),
            homepage: String::from(""),
            license: String::from("MIT"),
        };
        pm.available.push(meta);
        let results = pm.search_packages("example");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_dependencies() {
        let mut pm = PackageManager::new();
        let dep = PackageDependency {
            name: String::from("libfoo"),
            version_min: String::from("1.0.0"),
            version_max: String::from("2.0.0"),
            optional: false,
        };
        let meta = PackageMetadata {
            id: String::from("test"),
            name: String::from("Test"),
            version: String::from("1.0.0"),
            author: String::from("Test"),
            description: String::from("Test"),
            category: String::from("Utility"),
            size_bytes: 1024,
            dependencies: alloc::vec![dep],
            homepage: String::from(""),
            license: String::from("MIT"),
        };
        assert!(!pm.check_dependencies(&meta));
    }
}
