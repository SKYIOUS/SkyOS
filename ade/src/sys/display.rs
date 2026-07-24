// Scaffold — used by future phase
#![allow(dead_code)]
//! Display Manager subsystem — resolution, multiple displays, arrangement, scaling, rotation.
//!
//! Manages display configurations, hot-plug detection, and multi-monitor support.
//! Provides abstraction for future GPU backend compatibility.

use alloc::vec::Vec;

/// Display orientation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayOrientation {
    Normal,      // 0°
    Rotated90,   // 90°
    Rotated180,  // 180°
    Rotated270,  // 270°
}

/// Display scaling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalingMode {
    /// 1:1 pixel mapping
    None,
    /// Integer scaling (2x, 3x, etc.)
    Integer,
    /// Aspect-ratio preserving
    AspectRatio,
    /// Fill entire display
    Fill,
}

/// Refresh rate
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefreshRate {
    pub numerator: u16,
    pub denominator: u16,
}

impl RefreshRate {
    pub fn hz(&self) -> u16 {
        self.numerator / self.denominator
    }
}

/// Display resolution
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolution {
    pub width: u16,
    pub height: u16,
}

/// Display device information
#[derive(Clone, Copy, Debug)]
pub struct DisplayInfo {
    pub id: u32,
    pub connected: bool,
    pub width_mm: u16,
    pub height_mm: u16,
}

/// Display mode (resolution + refresh rate)
#[derive(Clone, Copy, Debug)]
pub struct DisplayMode {
    pub resolution: Resolution,
    pub refresh_rate: RefreshRate,
}

/// Display configuration
#[derive(Clone)]
pub struct DisplayConfig {
    pub id: u32,
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
    pub mode: DisplayMode,
    pub orientation: DisplayOrientation,
    pub scaling: ScalingMode,
    pub scale_factor: f32,
}

/// Display hot-plug event
#[derive(Clone, Copy, Debug)]
pub enum DisplayEvent {
    /// Display connected
    Connected(u32),
    /// Display disconnected
    Disconnected(u32),
    /// Resolution changed
    ModeChanged(u32),
}

/// Display Manager
pub struct DisplayManager {
    /// Available displays
    displays: Vec<DisplayConfig>,
    /// Primary display ID
    primary_id: u32,
    /// Display info cache
    display_info: Vec<DisplayInfo>,
    /// Pending events
    events: Vec<DisplayEvent>,
}

impl DisplayManager {
    /// Create a new display manager
    pub fn new(width: u16, height: u16) -> Self {
        let mut manager = DisplayManager {
            displays: Vec::new(),
            primary_id: 0,
            display_info: Vec::new(),
            events: Vec::new(),
        };

        // Create default display configuration
        let display = DisplayConfig {
            id: 0,
            enabled: true,
            x: 0,
            y: 0,
            mode: DisplayMode {
                resolution: Resolution { width, height },
                refresh_rate: RefreshRate {
                    numerator: 60,
                    denominator: 1,
                },
            },
            orientation: DisplayOrientation::Normal,
            scaling: ScalingMode::None,
            scale_factor: 1.0,
        };

        manager.displays.push(display);
        manager
    }

    /// Get number of connected displays
    pub fn display_count(&self) -> usize {
        self.displays.iter().filter(|d| d.enabled).count()
    }

    /// Get display configuration by ID
    pub fn get_display(&self, id: u32) -> Option<&DisplayConfig> {
        self.displays.iter().find(|d| d.id == id)
    }

    /// Get mutable display configuration
    pub fn get_display_mut(&mut self, id: u32) -> Option<&mut DisplayConfig> {
        self.displays.iter_mut().find(|d| d.id == id)
    }

    /// Get primary display
    pub fn primary_display(&self) -> Option<&DisplayConfig> {
        self.get_display(self.primary_id)
    }

    /// Get all displays
    pub fn all_displays(&self) -> &[DisplayConfig] {
        &self.displays
    }

    /// Set display resolution
    pub fn set_resolution(&mut self, id: u32, width: u16, height: u16) -> bool {
        if let Some(display) = self.get_display_mut(id) {
            display.mode.resolution = Resolution { width, height };
            self.events.push(DisplayEvent::ModeChanged(id));
            true
        } else {
            false
        }
    }

    /// Set display refresh rate
    pub fn set_refresh_rate(&mut self, id: u32, hz: u16) -> bool {
        if let Some(display) = self.get_display_mut(id) {
            display.mode.refresh_rate = RefreshRate {
                numerator: hz,
                denominator: 1,
            };
            self.events.push(DisplayEvent::ModeChanged(id));
            true
        } else {
            false
        }
    }

    /// Set display position (for multi-monitor)
    pub fn set_position(&mut self, id: u32, x: i32, y: i32) -> bool {
        if let Some(display) = self.get_display_mut(id) {
            display.x = x;
            display.y = y;
            true
        } else {
            false
        }
    }

    /// Set display orientation/rotation
    pub fn set_orientation(&mut self, id: u32, orientation: DisplayOrientation) -> bool {
        if let Some(display) = self.get_display_mut(id) {
            display.orientation = orientation;
            self.events.push(DisplayEvent::ModeChanged(id));
            true
        } else {
            false
        }
    }

    /// Set scaling mode and factor
    pub fn set_scaling(&mut self, id: u32, mode: ScalingMode, factor: f32) -> bool {
        if let Some(display) = self.get_display_mut(id) {
            display.scaling = mode;
            display.scale_factor = factor.max(0.5).min(4.0);
            true
        } else {
            false
        }
    }

    /// Enable/disable display
    pub fn set_enabled(&mut self, id: u32, enabled: bool) -> bool {
        if let Some(display) = self.get_display_mut(id) {
            display.enabled = enabled;
            if enabled {
                self.events.push(DisplayEvent::Connected(id));
            } else {
                self.events.push(DisplayEvent::Disconnected(id));
            }
            true
        } else {
            false
        }
    }

    /// Set primary display
    pub fn set_primary(&mut self, id: u32) -> bool {
        if self.displays.iter().any(|d| d.id == id) {
            self.primary_id = id;
            true
        } else {
            false
        }
    }

    /// Register a new display (hot-plug)
    pub fn register_display(&mut self, info: DisplayInfo) {
        if !self.display_info.iter().any(|d| d.id == info.id) {
            let config = DisplayConfig {
                id: info.id,
                enabled: false,
                x: 0,
                y: 0,
                mode: DisplayMode {
                    resolution: Resolution {
                        width: 1920,
                        height: 1080,
                    },
                    refresh_rate: RefreshRate {
                        numerator: 60,
                        denominator: 1,
                    },
                },
                orientation: DisplayOrientation::Normal,
                scaling: ScalingMode::None,
                scale_factor: 1.0,
            };
            self.displays.push(config);
            self.display_info.push(info);
            self.events.push(DisplayEvent::Connected(info.id));
        }
    }

    /// Unregister a display (hot-unplug)
    pub fn unregister_display(&mut self, id: u32) {
        self.displays.retain(|d| d.id != id);
        self.display_info.retain(|d| d.id != id);
        if self.primary_id == id && !self.displays.is_empty() {
            self.primary_id = self.displays[0].id;
        }
        self.events.push(DisplayEvent::Disconnected(id));
    }

    /// Drain pending display events
    pub fn drain_events(&mut self) -> Vec<DisplayEvent> {
        let events = self.events.clone();
        self.events.clear();
        events
    }

    /// Get total virtual screen size (bounding box of all displays)
    pub fn virtual_size(&self) -> (i32, i32) {
        let mut max_x = 0i32;
        let mut max_y = 0i32;

        for display in &self.displays {
            if display.enabled {
                let right = display.x + display.mode.resolution.width as i32;
                let bottom = display.y + display.mode.resolution.height as i32;
                max_x = max_x.max(right);
                max_y = max_y.max(bottom);
            }
        }

        (max_x, max_y)
    }

    /// Check if point is on a display
    pub fn point_on_display(&self, x: i32, y: i32) -> Option<u32> {
        for display in &self.displays {
            if display.enabled {
                let dx = x - display.x;
                let dy = y - display.y;
                let width = display.mode.resolution.width as i32;
                let height = display.mode.resolution.height as i32;

                if dx >= 0 && dy >= 0 && dx < width && dy < height {
                    return Some(display.id);
                }
            }
        }
        None
    }

    /// Mirror displays (clone mode)
    pub fn mirror_displays(&mut self, source_id: u32, target_id: u32) -> bool {
        if let Some(source) = self.get_display(source_id) {
            let mode = source.mode;
            if let Some(target) = self.get_display_mut(target_id) {
                target.mode = mode;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Extend displays (side-by-side)
    pub fn extend_display(&mut self, target_id: u32, next_to_id: u32, to_right: bool) -> bool {
        let (target_width, target_x, target_y) = if let Some(target) = self.get_display(target_id) {
            (
                target.mode.resolution.width as i32,
                target.x,
                target.y,
            )
        } else {
            return false;
        };

        if let Some(next) = self.get_display_mut(next_to_id) {
            if to_right {
                next.x = target_x + target_width;
                next.y = target_y;
            } else {
                next.x = target_x - (next.mode.resolution.width as i32);
                next.y = target_y;
            }
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_manager_creation() {
        let dm = DisplayManager::new(1920, 1080);
        assert_eq!(dm.display_count(), 1);
    }

    #[test]
    fn test_set_resolution() {
        let mut dm = DisplayManager::new(1920, 1080);
        assert!(dm.set_resolution(0, 2560, 1440));
        let display = dm.get_display(0).unwrap();
        assert_eq!(display.mode.resolution.width, 2560);
    }

    #[test]
    fn test_point_on_display() {
        let dm = DisplayManager::new(1920, 1080);
        assert_eq!(dm.point_on_display(100, 100), Some(0));
        assert_eq!(dm.point_on_display(2000, 100), None);
    }

    #[test]
    fn test_virtual_size() {
        let dm = DisplayManager::new(1920, 1080);
        let (width, height) = dm.virtual_size();
        assert_eq!(width, 1920);
        assert_eq!(height, 1080);
    }
}
