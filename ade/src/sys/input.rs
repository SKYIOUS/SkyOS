// Scaffold — used by future phase
#![allow(dead_code)]
//! Input Device Manager — keyboard layouts, mouse settings, touchpad, gamepad, accessibility.
//!
//! Manages input devices, keyboard layouts, and input accessibility settings.
//! Provides abstraction for future device drivers and advanced input capabilities.

use alloc::vec::Vec;
use alloc::string::String;

/// Input device type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputDeviceType {
    Keyboard,
    Mouse,
    Touchpad,
    Touchscreen,
    Gamepad,
    Joystick,
    Tablet,
    Stylus,
}

/// Keyboard layout identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardLayout {
    UsQwerty,
    EuQwerty,
    Dvorak,
    Colemak,
    ErgoDox,
    Custom,
}

/// Mouse button mapping
#[derive(Clone, Copy, Debug)]
pub struct MouseButtons {
    pub left: u8,
    pub middle: u8,
    pub right: u8,
}

/// Accessibility settings
#[derive(Clone, Copy, Debug)]
pub struct AccessibilitySettings {
    pub sticky_keys_enabled: bool,
    pub slow_keys_enabled: bool,
    pub bounce_keys_enabled: bool,
    pub mouse_keys_enabled: bool,
    pub high_contrast: bool,
    pub large_text: bool,
    pub screen_magnifier: bool,
    pub screen_reader: bool,
}

impl Default for AccessibilitySettings {
    fn default() -> Self {
        AccessibilitySettings {
            sticky_keys_enabled: false,
            slow_keys_enabled: false,
            bounce_keys_enabled: false,
            mouse_keys_enabled: false,
            high_contrast: false,
            large_text: false,
            screen_magnifier: false,
            screen_reader: false,
        }
    }
}

/// Input device configuration
#[derive(Clone)]
pub struct InputDevice {
    pub id: u32,
    pub name: String,
    pub device_type: InputDeviceType,
    pub enabled: bool,
}

/// Keyboard settings
#[derive(Clone, Copy, Debug)]
pub struct KeyboardSettings {
    pub layout: KeyboardLayout,
    pub repeat_enabled: bool,
    pub repeat_delay_ms: u16,
    pub repeat_rate_ms: u16,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        KeyboardSettings {
            layout: KeyboardLayout::UsQwerty,
            repeat_enabled: true,
            repeat_delay_ms: 500,
            repeat_rate_ms: 50,
        }
    }
}

/// Mouse settings
#[derive(Clone, Copy, Debug)]
pub struct MouseSettings {
    pub accel_enabled: bool,
    pub accel_profile: u8,     // 0=flat, 1=adaptive, 2=custom
    pub sensitivity: u8,        // 1-100
    pub double_click_time_ms: u16,
    pub button_map: MouseButtons,
    pub natural_scroll: bool,
}

impl Default for MouseSettings {
    fn default() -> Self {
        MouseSettings {
            accel_enabled: true,
            accel_profile: 1,
            sensitivity: 50,
            double_click_time_ms: 500,
            button_map: MouseButtons {
                left: 1,
                middle: 2,
                right: 3,
            },
            natural_scroll: false,
        }
    }
}

/// Touchpad settings
#[derive(Clone, Copy, Debug)]
pub struct TouchpadSettings {
    pub enabled: bool,
    pub tap_to_click: bool,
    pub two_finger_scroll: bool,
    pub edge_scroll: bool,
    pub natural_scroll: bool,
}

impl Default for TouchpadSettings {
    fn default() -> Self {
        TouchpadSettings {
            enabled: true,
            tap_to_click: true,
            two_finger_scroll: true,
            edge_scroll: false,
            natural_scroll: false,
        }
    }
}

/// Gamepad button state
#[derive(Clone, Copy, Debug)]
pub struct GamepadState {
    pub left_stick_x: i16,
    pub left_stick_y: i16,
    pub right_stick_x: i16,
    pub right_stick_y: i16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub button_a: bool,
    pub button_b: bool,
    pub button_x: bool,
    pub button_y: bool,
}

/// Input Device Manager
pub struct InputManager {
    /// Input devices
    devices: Vec<InputDevice>,
    /// Keyboard settings
    keyboard_settings: KeyboardSettings,
    /// Mouse settings
    mouse_settings: MouseSettings,
    /// Touchpad settings
    touchpad_settings: TouchpadSettings,
    /// Accessibility settings
    accessibility: AccessibilitySettings,
}

impl InputManager {
    /// Create a new input manager
    pub fn new() -> Self {
        let mut manager = InputManager {
            devices: Vec::new(),
            keyboard_settings: KeyboardSettings::default(),
            mouse_settings: MouseSettings::default(),
            touchpad_settings: TouchpadSettings::default(),
            accessibility: AccessibilitySettings::default(),
        };

        // Register default input devices
        manager.devices.push(InputDevice {
            id: 0,
            name: String::from("Keyboard"),
            device_type: InputDeviceType::Keyboard,
            enabled: true,
        });

        manager.devices.push(InputDevice {
            id: 1,
            name: String::from("Mouse"),
            device_type: InputDeviceType::Mouse,
            enabled: true,
        });

        manager.devices.push(InputDevice {
            id: 2,
            name: String::from("Touchpad"),
            device_type: InputDeviceType::Touchpad,
            enabled: true,
        });

        manager
    }

    /// Get device by ID
    pub fn get_device(&self, id: u32) -> Option<&InputDevice> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// Get mutable device
    pub fn get_device_mut(&mut self, id: u32) -> Option<&mut InputDevice> {
        self.devices.iter_mut().find(|d| d.id == id)
    }

    /// Register a new input device
    pub fn register_device(&mut self, name: &str, device_type: InputDeviceType) -> u32 {
        let id = self.devices.iter().map(|d| d.id).max().unwrap_or(0) + 1;

        self.devices.push(InputDevice {
            id,
            name: String::from(name),
            device_type,
            enabled: true,
        });

        id
    }

    /// Unregister a device
    pub fn unregister_device(&mut self, id: u32) {
        self.devices.retain(|d| d.id != id);
    }

    /// Get keyboard settings
    pub fn keyboard_settings(&self) -> &KeyboardSettings {
        &self.keyboard_settings
    }

    /// Set keyboard layout
    pub fn set_keyboard_layout(&mut self, layout: KeyboardLayout) {
        self.keyboard_settings.layout = layout;
    }

    /// Set keyboard repeat settings
    pub fn set_key_repeat(&mut self, enabled: bool, delay_ms: u16, rate_ms: u16) {
        self.keyboard_settings.repeat_enabled = enabled;
        self.keyboard_settings.repeat_delay_ms = delay_ms;
        self.keyboard_settings.repeat_rate_ms = rate_ms;
    }

    /// Get mouse settings
    pub fn mouse_settings(&self) -> &MouseSettings {
        &self.mouse_settings
    }

    /// Set mouse sensitivity
    pub fn set_mouse_sensitivity(&mut self, sensitivity: u8) {
        self.mouse_settings.sensitivity = sensitivity.min(100).max(1);
    }

    /// Set mouse acceleration
    pub fn set_mouse_acceleration(&mut self, enabled: bool, profile: u8) {
        self.mouse_settings.accel_enabled = enabled;
        self.mouse_settings.accel_profile = profile.min(2);
    }

    /// Set double-click time
    pub fn set_double_click_time(&mut self, time_ms: u16) {
        self.mouse_settings.double_click_time_ms = time_ms;
    }

    /// Swap mouse buttons (for left-handed users)
    pub fn set_left_handed(&mut self, left_handed: bool) {
        if left_handed {
            self.mouse_settings.button_map = MouseButtons {
                left: 3,
                middle: 2,
                right: 1,
            };
        } else {
            self.mouse_settings.button_map = MouseButtons {
                left: 1,
                middle: 2,
                right: 3,
            };
        }
    }

    /// Get touchpad settings
    pub fn touchpad_settings(&self) -> &TouchpadSettings {
        &self.touchpad_settings
    }

    /// Enable/disable touchpad
    pub fn set_touchpad_enabled(&mut self, enabled: bool) {
        self.touchpad_settings.enabled = enabled;
    }

    /// Set tap-to-click
    pub fn set_tap_to_click(&mut self, enabled: bool) {
        self.touchpad_settings.tap_to_click = enabled;
    }

    /// Set two-finger scroll
    pub fn set_two_finger_scroll(&mut self, enabled: bool) {
        self.touchpad_settings.two_finger_scroll = enabled;
    }

    /// Set natural scroll (Mac-style)
    pub fn set_natural_scroll(&mut self, device_type: InputDeviceType, enabled: bool) {
        match device_type {
            InputDeviceType::Mouse => self.mouse_settings.natural_scroll = enabled,
            InputDeviceType::Touchpad => self.touchpad_settings.natural_scroll = enabled,
            _ => {}
        }
    }

    /// Get accessibility settings
    pub fn accessibility(&self) -> &AccessibilitySettings {
        &self.accessibility
    }

    /// Get mutable accessibility settings
    pub fn accessibility_mut(&mut self) -> &mut AccessibilitySettings {
        &mut self.accessibility
    }

    /// Enable sticky keys (accessibility)
    pub fn set_sticky_keys(&mut self, enabled: bool) {
        self.accessibility.sticky_keys_enabled = enabled;
    }

    /// Enable slow keys (accessibility)
    pub fn set_slow_keys(&mut self, enabled: bool) {
        self.accessibility.slow_keys_enabled = enabled;
    }

    /// Enable bounce keys (accessibility)
    pub fn set_bounce_keys(&mut self, enabled: bool) {
        self.accessibility.bounce_keys_enabled = enabled;
    }

    /// Enable mouse keys (control mouse from keyboard)
    pub fn set_mouse_keys(&mut self, enabled: bool) {
        self.accessibility.mouse_keys_enabled = enabled;
    }

    /// Enable high contrast mode
    pub fn set_high_contrast(&mut self, enabled: bool) {
        self.accessibility.high_contrast = enabled;
    }

    /// Enable large text
    pub fn set_large_text(&mut self, enabled: bool) {
        self.accessibility.large_text = enabled;
    }

    /// Enable screen magnifier
    pub fn set_magnifier(&mut self, enabled: bool) {
        self.accessibility.screen_magnifier = enabled;
    }

    /// Enable screen reader
    pub fn set_screen_reader(&mut self, enabled: bool) {
        self.accessibility.screen_reader = enabled;
    }

    /// Get all input devices
    pub fn devices(&self) -> &[InputDevice] {
        &self.devices
    }

    /// Find device by type
    pub fn find_device(&self, device_type: InputDeviceType) -> Option<&InputDevice> {
        self.devices.iter().find(|d| d.device_type == device_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_manager_creation() {
        let im = InputManager::new();
        assert!(im.devices().len() >= 3);
    }

    #[test]
    fn test_keyboard_layout() {
        let mut im = InputManager::new();
        im.set_keyboard_layout(KeyboardLayout::Dvorak);
        assert_eq!(im.keyboard_settings().layout, KeyboardLayout::Dvorak);
    }

    #[test]
    fn test_mouse_sensitivity() {
        let mut im = InputManager::new();
        im.set_mouse_sensitivity(75);
        assert_eq!(im.mouse_settings().sensitivity, 75);
    }

    #[test]
    fn test_left_handed() {
        let mut im = InputManager::new();
        im.set_left_handed(true);
        assert_eq!(im.mouse_settings().button_map.left, 3);
    }

    #[test]
    fn test_accessibility() {
        let mut im = InputManager::new();
        im.set_sticky_keys(true);
        assert!(im.accessibility().sticky_keys_enabled);
    }
}
