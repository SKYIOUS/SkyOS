//! SkyOS version information
//!
//! This module provides centralized version management for the entire OS.
//! All components should reference this file for version information.

/// Current SkyOS version
pub const SKYOS_VERSION: &str = "0.0.1";

/// Current kernel version
pub const KERNEL_VERSION: &str = "0.0.1";

/// Current userspace version
pub const USERSPACE_VERSION: &str = "0.0.1";

/// Update repository URL
pub const UPDATE_REPO_URL: &str = "https://raw.githubusercontent.com/SKYIOUS/sarga-updates/main";

/// Update manifest filename
pub const UPDATE_MANIFEST: &str = "update.toml";

/// Get current version as a formatted string
pub fn get_version_string() -> alloc::string::String {
    alloc::format!("SkyOS v{}", SKYOS_VERSION)
}

/// Get detailed version information
pub fn get_version_info() -> alloc::string::String {
    alloc::format!(
        "SkyOS v{}\nKernel: v{}\nUserspace: v{}",
        SKYOS_VERSION,
        KERNEL_VERSION,
        USERSPACE_VERSION
    )
}

/// Get full update manifest URL
pub fn get_update_manifest_url() -> alloc::string::String {
    alloc::format!("{}/{}", UPDATE_REPO_URL, UPDATE_MANIFEST)
}
