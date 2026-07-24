// Scaffold — used by future phase
#![allow(dead_code)]
//! Plugin System — plugin API, lifecycle, versioning, capabilities, sandbox.
//!
//! Manages plugin loading, lifecycle, capabilities, dependencies, and safe execution.
//! Provides a complete plugin ecosystem for desktop extensions.

use alloc::string::String;
use alloc::vec::Vec;

/// Plugin version
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        Version { major, minor, patch }
    }

    pub fn is_compatible_with(&self, requirement: Version) -> bool {
        self.major == requirement.major && self.minor >= requirement.minor
    }
}

/// Plugin capability flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum PluginCapability {
    /// Read file system
    FileRead = 1 << 0,
    /// Write file system
    FileWrite = 1 << 1,
    /// Access network
    Network = 1 << 2,
    /// Access audio
    Audio = 1 << 3,
    /// Access input devices
    Input = 1 << 4,
    /// Create windows/UI
    Windowing = 1 << 5,
    /// Access clipboard
    Clipboard = 1 << 6,
    /// System information
    SystemInfo = 1 << 7,
    /// Create processes
    ProcessManagement = 1 << 8,
    /// Access device
    DeviceAccess = 1 << 9,
}

/// Plugin lifecycle state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginState {
    /// Not loaded
    Unloaded,
    /// Loading...
    Loading,
    /// Loaded and ready
    Loaded,
    /// Initialized and running
    Active,
    /// Paused
    Paused,
    /// Unloading...
    Unloading,
    /// Failed state
    Failed,
}

/// Plugin dependency
#[derive(Clone)]
pub struct PluginDependency {
    pub name: String,
    pub version_requirement: Version,
}

/// Plugin metadata
#[derive(Clone)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: Version,
    pub author: String,
    pub description: String,
    pub capabilities: u32,
    pub dependencies: Vec<PluginDependency>,
    pub entry_point: String,
}

/// Plugin instance
pub struct Plugin {
    pub metadata: PluginMetadata,
    pub state: PluginState,
    pub sandbox_id: u32,
    pub load_time_ms: u32,
    pub error: Option<String>,
}

/// Plugin sandbox configuration
pub struct SandboxConfig {
    pub id: u32,
    pub capabilities: u32,
    pub memory_limit_kb: u32,
    pub cpu_time_limit_ms: u32,
    pub isolated: bool,
}

/// Plugin event
#[derive(Clone, Copy, Debug)]
pub enum PluginEvent {
    /// Plugin loaded successfully
    Loaded(u32),
    /// Plugin failed to load
    LoadFailed(u32),
    /// Plugin activated
    Activated(u32),
    /// Plugin deactivated
    Deactivated(u32),
    /// Plugin crashed
    Crashed(u32),
    /// Plugin requested capability
    CapabilityRequested(u32, u32),
}

/// Plugin Manager
pub struct PluginManager {
    /// Loaded plugins
    plugins: Vec<Plugin>,
    /// Available plugin metadata (discovered but not loaded)
    available: Vec<PluginMetadata>,
    /// Sandbox configurations
    sandboxes: Vec<SandboxConfig>,
    /// Pending events
    events: Vec<PluginEvent>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        PluginManager {
            plugins: Vec::new(),
            available: Vec::new(),
            sandboxes: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Discover plugins (scan plugin directory)
    pub fn discover_plugins(&mut self, metadata_list: Vec<PluginMetadata>) {
        for metadata in metadata_list {
            if !self.available.iter().any(|p| p.id == metadata.id) {
                self.available.push(metadata);
            }
        }
    }

    /// Get available plugins
    pub fn available_plugins(&self) -> &[PluginMetadata] {
        &self.available
    }

    /// Load a plugin
    pub fn load_plugin(&mut self, plugin_id: &str) -> bool {
        // Check if already loaded
        if self.plugins.iter().any(|p| p.metadata.id == plugin_id) {
            return false;
        }

        // Find metadata
        let metadata = if let Some(m) = self.available.iter().find(|p| p.id == plugin_id) {
            m.clone()
        } else {
            return false;
        };

        // Check dependencies
        for dep in &metadata.dependencies {
            if !self.is_plugin_loaded(&dep.name) {
                self.events.push(PluginEvent::LoadFailed(0));
                return false;
            }
        }

        // Create sandbox
        let sandbox_id = self.sandboxes.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        self.sandboxes.push(SandboxConfig {
            id: sandbox_id,
            capabilities: metadata.capabilities,
            memory_limit_kb: 16384,
            cpu_time_limit_ms: 5000,
            isolated: true,
        });

        // Create plugin instance
        let plugin = Plugin {
            metadata: metadata.clone(),
            state: PluginState::Loaded,
            sandbox_id,
            load_time_ms: 0,
            error: None,
        };

        self.plugins.push(plugin);
        self.events.push(PluginEvent::Loaded(sandbox_id));
        true
    }

    /// Unload a plugin
    pub fn unload_plugin(&mut self, plugin_id: &str) -> bool {
        if let Some(pos) = self.plugins.iter().position(|p| p.metadata.id == plugin_id) {
            let plugin = self.plugins.remove(pos);
            self.sandboxes.retain(|s| s.id != plugin.sandbox_id);
            self.events.push(PluginEvent::Deactivated(plugin.sandbox_id));
            true
        } else {
            false
        }
    }

    /// Check if plugin is loaded
    pub fn is_plugin_loaded(&self, plugin_id: &str) -> bool {
        self.plugins.iter().any(|p| p.metadata.id == plugin_id && p.state != PluginState::Failed)
    }

    /// Get plugin by ID
    pub fn get_plugin(&self, plugin_id: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|p| p.metadata.id == plugin_id)
    }

    /// Get mutable plugin
    pub fn get_plugin_mut(&mut self, plugin_id: &str) -> Option<&mut Plugin> {
        self.plugins.iter_mut().find(|p| p.metadata.id == plugin_id)
    }

    /// Activate a loaded plugin
    pub fn activate_plugin(&mut self, plugin_id: &str) -> bool {
        let sandbox_id = if let Some(plugin) = self.get_plugin_mut(plugin_id) {
            if plugin.state == PluginState::Loaded {
                plugin.state = PluginState::Active;
                Some(plugin.sandbox_id)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(id) = sandbox_id {
            self.events.push(PluginEvent::Activated(id));
            true
        } else {
            false
        }
    }

    /// Deactivate a running plugin
    pub fn deactivate_plugin(&mut self, plugin_id: &str) -> bool {
        let sandbox_id = if let Some(plugin) = self.get_plugin_mut(plugin_id) {
            if plugin.state == PluginState::Active {
                plugin.state = PluginState::Loaded;
                Some(plugin.sandbox_id)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(id) = sandbox_id {
            self.events.push(PluginEvent::Deactivated(id));
            true
        } else {
            false
        }
    }

    /// Pause a running plugin
    pub fn pause_plugin(&mut self, plugin_id: &str) -> bool {
        if let Some(plugin) = self.get_plugin_mut(plugin_id) {
            if plugin.state == PluginState::Active {
                plugin.state = PluginState::Paused;
                return true;
            }
        }
        false
    }

    /// Resume a paused plugin
    pub fn resume_plugin(&mut self, plugin_id: &str) -> bool {
        if let Some(plugin) = self.get_plugin_mut(plugin_id) {
            if plugin.state == PluginState::Paused {
                plugin.state = PluginState::Active;
                return true;
            }
        }
        false
    }

    /// Mark plugin as failed
    pub fn mark_plugin_failed(&mut self, plugin_id: &str, error: &str) {
        let sandbox_id = if let Some(plugin) = self.get_plugin_mut(plugin_id) {
            plugin.state = PluginState::Failed;
            plugin.error = Some(String::from(error));
            Some(plugin.sandbox_id)
        } else {
            None
        };

        if let Some(id) = sandbox_id {
            self.events.push(PluginEvent::Crashed(id));
        }
    }

    /// Check if plugin has capability
    pub fn check_capability(&self, plugin_id: &str, capability: PluginCapability) -> bool {
        if let Some(plugin) = self.get_plugin(plugin_id) {
            (plugin.metadata.capabilities & (capability as u32)) != 0
        } else {
            false
        }
    }

    /// Request new capability
    pub fn request_capability(&mut self, plugin_id: &str, capability: PluginCapability) -> bool {
        if let Some(plugin) = self.get_plugin(plugin_id) {
            self.events.push(PluginEvent::CapabilityRequested(plugin.sandbox_id, capability as u32));
            true
        } else {
            false
        }
    }

    /// Get all loaded plugins
    pub fn loaded_plugins(&self) -> Vec<&Plugin> {
        self.plugins
            .iter()
            .filter(|p| p.state != PluginState::Failed)
            .collect()
    }

    /// Get active plugins
    pub fn active_plugins(&self) -> Vec<&Plugin> {
        self.plugins
            .iter()
            .filter(|p| p.state == PluginState::Active)
            .collect()
    }

    /// Drain pending plugin events
    pub fn drain_events(&mut self) -> Vec<PluginEvent> {
        let events = self.events.clone();
        self.events.clear();
        events
    }

    /// Hot-reload a plugin
    pub fn hot_reload_plugin(&mut self, plugin_id: &str) -> bool {
        let was_active = self
            .get_plugin(plugin_id)
            .map(|p| p.state == PluginState::Active)
            .unwrap_or(false);

        if self.unload_plugin(plugin_id) && self.load_plugin(plugin_id) {
            if was_active {
                self.activate_plugin(plugin_id)
            } else {
                true
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_creation() {
        let pm = PluginManager::new();
        assert_eq!(pm.loaded_plugins().len(), 0);
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 0);
        assert!(v1.is_compatible_with(v2));
    }

    #[test]
    fn test_plugin_lifecycle() {
        let mut pm = PluginManager::new();
        let metadata = PluginMetadata {
            id: String::from("test-plugin"),
            name: String::from("Test"),
            version: Version::new(1, 0, 0),
            author: String::from("Test"),
            description: String::from("Test plugin"),
            capabilities: PluginCapability::FileRead as u32,
            dependencies: Vec::new(),
            entry_point: String::from("main"),
        };

        pm.discover_plugins(alloc::vec![metadata]);
        assert!(pm.load_plugin("test-plugin"));
        assert!(pm.activate_plugin("test-plugin"));
        assert!(pm.deactivate_plugin("test-plugin"));
        assert!(pm.unload_plugin("test-plugin"));
    }

    #[test]
    fn test_capability_checking() {
        let mut pm = PluginManager::new();
        let metadata = PluginMetadata {
            id: String::from("test"),
            name: String::from("Test"),
            version: Version::new(1, 0, 0),
            author: String::from("Test"),
            description: String::from("Test"),
            capabilities: PluginCapability::FileRead as u32 | PluginCapability::Audio as u32,
            dependencies: Vec::new(),
            entry_point: String::from("main"),
        };

        pm.discover_plugins(alloc::vec![metadata]);
        pm.load_plugin("test");
        assert!(pm.check_capability("test", PluginCapability::FileRead));
        assert!(pm.check_capability("test", PluginCapability::Audio));
        assert!(!pm.check_capability("test", PluginCapability::Network));
    }
}
