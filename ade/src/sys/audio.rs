// Scaffold — used by future phase
#![allow(dead_code)]
//! Audio Framework — output devices, input devices, volume mixer, per-application volume.
//!
//! Manages audio devices, volume control, and mixer settings.
//! Provides abstraction for future PipeWire and hardware backend compatibility.

use alloc::vec::Vec;

/// Audio device type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioDeviceType {
    /// Speaker, headphones, etc.
    Output,
    /// Microphone, line-in, etc.
    Input,
}

/// Audio device state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceState {
    /// Device is available and active
    Active,
    /// Device is available but idle
    Idle,
    /// Device is unavailable (unplugged, etc.)
    Unavailable,
}

/// Audio device information
#[derive(Clone)]
pub struct AudioDevice {
    pub id: u32,
    pub name: alloc::string::String,
    pub device_type: AudioDeviceType,
    pub state: DeviceState,
    pub volume: u8, // 0-100
    pub muted: bool,
    pub is_default: bool,
}

/// Per-application audio settings
#[derive(Clone, Copy, Debug)]
pub struct AppAudioSettings {
    pub pid: u32,
    pub volume: u8, // 0-100
    pub muted: bool,
}

/// Audio event type
#[derive(Clone, Copy, Debug)]
pub enum AudioEvent {
    /// Device connected
    DeviceConnected(u32),
    /// Device disconnected
    DeviceDisconnected(u32),
    /// Default output device changed
    DefaultOutputChanged(u32),
    /// Default input device changed
    DefaultInputChanged(u32),
    /// Volume changed (device_id, new_volume)
    VolumeChanged(u32, u8),
    /// Mute state changed (device_id, muted)
    MuteChanged(u32, bool),
}

/// Audio Mixer
pub struct AudioMixer {
    /// All audio devices
    devices: Vec<AudioDevice>,
    /// Default output device
    default_output: u32,
    /// Default input device
    default_input: u32,
    /// Per-application volume settings
    app_settings: Vec<AppAudioSettings>,
    /// Master volume
    master_volume: u8,
    /// Pending events
    events: Vec<AudioEvent>,
}

impl AudioMixer {
    /// Create a new audio mixer
    pub fn new() -> Self {
        let mut mixer = AudioMixer {
            devices: Vec::new(),
            default_output: 0,
            default_input: 0,
            app_settings: Vec::new(),
            master_volume: 75,
            events: Vec::new(),
        };

        // Create default output device
        mixer.devices.push(AudioDevice {
            id: 0,
            name: alloc::string::String::from("Speaker"),
            device_type: AudioDeviceType::Output,
            state: DeviceState::Active,
            volume: 75,
            muted: false,
            is_default: true,
        });

        // Create default input device
        mixer.devices.push(AudioDevice {
            id: 1,
            name: alloc::string::String::from("Microphone"),
            device_type: AudioDeviceType::Input,
            state: DeviceState::Idle,
            volume: 80,
            muted: false,
            is_default: true,
        });

        mixer
    }

    /// Get number of devices
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Get device by ID
    pub fn get_device(&self, id: u32) -> Option<&AudioDevice> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// Get mutable device
    pub fn get_device_mut(&mut self, id: u32) -> Option<&mut AudioDevice> {
        self.devices.iter_mut().find(|d| d.id == id)
    }

    /// Get default output device
    pub fn default_output_device(&self) -> Option<&AudioDevice> {
        self.get_device(self.default_output)
    }

    /// Get default input device
    pub fn default_input_device(&self) -> Option<&AudioDevice> {
        self.get_device(self.default_input)
    }

    /// Get all output devices
    pub fn output_devices(&self) -> Vec<&AudioDevice> {
        self.devices
            .iter()
            .filter(|d| d.device_type == AudioDeviceType::Output)
            .collect()
    }

    /// Get all input devices
    pub fn input_devices(&self) -> Vec<&AudioDevice> {
        self.devices
            .iter()
            .filter(|d| d.device_type == AudioDeviceType::Input)
            .collect()
    }

    /// Register a new audio device
    pub fn register_device(&mut self, name: &str, device_type: AudioDeviceType) -> u32 {
        let id = self.devices.iter().map(|d| d.id).max().unwrap_or(0) + 1;

        self.devices.push(AudioDevice {
            id,
            name: alloc::string::String::from(name),
            device_type,
            state: DeviceState::Active,
            volume: 75,
            muted: false,
            is_default: self.devices.is_empty(),
        });

        if device_type == AudioDeviceType::Output && self.default_output == 0 {
            self.default_output = id;
        }
        if device_type == AudioDeviceType::Input && self.default_input == 0 {
            self.default_input = id;
        }

        self.events.push(AudioEvent::DeviceConnected(id));
        id
    }

    /// Unregister audio device
    pub fn unregister_device(&mut self, id: u32) {
        self.devices.retain(|d| d.id != id);
        if self.default_output == id {
            self.default_output = self
                .devices
                .iter()
                .find(|d| d.device_type == AudioDeviceType::Output)
                .map(|d| d.id)
                .unwrap_or(0);
        }
        if self.default_input == id {
            self.default_input = self
                .devices
                .iter()
                .find(|d| d.device_type == AudioDeviceType::Input)
                .map(|d| d.id)
                .unwrap_or(0);
        }
        self.events.push(AudioEvent::DeviceDisconnected(id));
    }

    /// Set device volume
    pub fn set_volume(&mut self, id: u32, volume: u8) -> bool {
        let volume = volume.min(100);
        if let Some(device) = self.get_device_mut(id) {
            device.volume = volume;
            self.events.push(AudioEvent::VolumeChanged(id, volume));
            true
        } else {
            false
        }
    }

    /// Set device mute state
    pub fn set_muted(&mut self, id: u32, muted: bool) -> bool {
        if let Some(device) = self.get_device_mut(id) {
            device.muted = muted;
            self.events.push(AudioEvent::MuteChanged(id, muted));
            true
        } else {
            false
        }
    }

    /// Toggle mute state
    pub fn toggle_mute(&mut self, id: u32) -> bool {
        if let Some(device) = self.get_device(id) {
            let new_muted = !device.muted;
            self.set_muted(id, new_muted)
        } else {
            false
        }
    }

    /// Set default output device
    pub fn set_default_output(&mut self, id: u32) -> bool {
        if let Some(device) = self.get_device(id) {
            if device.device_type == AudioDeviceType::Output {
                for d in &mut self.devices {
                    d.is_default = d.id == id;
                }
                self.default_output = id;
                self.events.push(AudioEvent::DefaultOutputChanged(id));
                return true;
            }
        }
        false
    }

    /// Set default input device
    pub fn set_default_input(&mut self, id: u32) -> bool {
        if let Some(device) = self.get_device(id) {
            if device.device_type == AudioDeviceType::Input {
                for d in &mut self.devices {
                    d.is_default = d.id == id;
                }
                self.default_input = id;
                self.events.push(AudioEvent::DefaultInputChanged(id));
                return true;
            }
        }
        false
    }

    /// Set per-application volume
    pub fn set_app_volume(&mut self, pid: u32, volume: u8) {
        let volume = volume.min(100);
        if let Some(settings) = self.app_settings.iter_mut().find(|s| s.pid == pid) {
            settings.volume = volume;
        } else {
            self.app_settings.push(AppAudioSettings {
                pid,
                volume,
                muted: false,
            });
        }
    }

    /// Get per-application volume
    pub fn get_app_volume(&self, pid: u32) -> u8 {
        self.app_settings
            .iter()
            .find(|s| s.pid == pid)
            .map(|s| s.volume)
            .unwrap_or(self.master_volume)
    }

    /// Set application mute state
    pub fn set_app_muted(&mut self, pid: u32, muted: bool) {
        if let Some(settings) = self.app_settings.iter_mut().find(|s| s.pid == pid) {
            settings.muted = muted;
        } else {
            self.app_settings.push(AppAudioSettings {
                pid,
                volume: self.master_volume,
                muted,
            });
        }
    }

    /// Get application mute state
    pub fn is_app_muted(&self, pid: u32) -> bool {
        self.app_settings
            .iter()
            .find(|s| s.pid == pid)
            .map(|s| s.muted)
            .unwrap_or(false)
    }

    /// Set master volume
    pub fn set_master_volume(&mut self, volume: u8) {
        self.master_volume = volume.min(100);
    }

    /// Get master volume
    pub fn master_volume(&self) -> u8 {
        self.master_volume
    }

    /// Remove application settings (when app terminates)
    pub fn remove_app(&mut self, pid: u32) {
        self.app_settings.retain(|s| s.pid != pid);
    }

    /// Drain pending audio events
    pub fn drain_events(&mut self) -> Vec<AudioEvent> {
        let events = self.events.clone();
        self.events.clear();
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_mixer_creation() {
        let mixer = AudioMixer::new();
        assert!(mixer.device_count() >= 2);
        assert!(mixer.default_output_device().is_some());
    }

    #[test]
    fn test_set_volume() {
        let mut mixer = AudioMixer::new();
        assert!(mixer.set_volume(0, 50));
        let device = mixer.get_device(0).unwrap();
        assert_eq!(device.volume, 50);
    }

    #[test]
    fn test_app_volume() {
        let mut mixer = AudioMixer::new();
        mixer.set_app_volume(1234, 60);
        assert_eq!(mixer.get_app_volume(1234), 60);
    }

    #[test]
    fn test_register_device() {
        let mut mixer = AudioMixer::new();
        let id = mixer.register_device("Line-Out", AudioDeviceType::Output);
        assert!(mixer.get_device(id).is_some());
    }
}
