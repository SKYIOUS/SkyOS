// Scaffold — used by future phase
#![allow(dead_code)]
//! Power Management subsystem — sleep, suspend, hibernate, shutdown, restart.
//!
//! Manages system power states, idle detection, screen blanking, and power events.
//! Provides a clean abstraction over hardware power operations.

use alloc::vec::Vec;

/// Power state of the system
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerState {
    /// System is running normally
    Active,
    /// CPU is idle but responsive (monitor on)
    Idle,
    /// Display is blanked, system responsive
    Standby,
    /// Minimal power consumption, can wake
    Suspend,
    /// Persisted to disk, hibernating
    Hibernate,
    /// System is shutting down
    Shutdown,
    /// System is restarting
    Restart,
}

/// Power event types
#[derive(Clone, Copy, Debug)]
pub enum PowerEvent {
    /// System is becoming idle
    IdleDetected,
    /// User activity detected
    ActivityDetected,
    /// Battery low
    BatteryLow,
    /// Battery critical
    BatteryCritical,
    /// Power adapter connected
    PowerConnected,
    /// Power adapter disconnected
    PowerDisconnected,
    /// Lid closed
    LidClosed,
    /// Lid opened
    LidOpened,
}

/// Battery information
#[derive(Clone, Copy, Debug)]
pub struct BatteryInfo {
    pub present: bool,
    pub percentage: u8,
    pub is_charging: bool,
    pub time_remaining_minutes: u16,
}

/// Power management configuration
#[derive(Clone)]
pub struct PowerConfig {
    /// Idle timeout before standby (seconds)
    pub idle_standby_timeout: u16,
    /// Idle timeout before suspend (seconds)
    pub idle_suspend_timeout: u16,
    /// Battery percentage threshold for "low" warning
    pub battery_low_threshold: u8,
    /// Battery percentage threshold for "critical" warning
    pub battery_critical_threshold: u8,
    /// Enable screen blanking on idle
    pub blank_screen_on_idle: bool,
}

impl Default for PowerConfig {
    fn default() -> Self {
        PowerConfig {
            idle_standby_timeout: 600,    // 10 minutes
            idle_suspend_timeout: 1800,   // 30 minutes
            battery_low_threshold: 20,
            battery_critical_threshold: 5,
            blank_screen_on_idle: true,
        }
    }
}

/// Power Management subsystem
pub struct PowerManager {
    /// Current power state
    current_state: PowerState,
    /// Configuration
    config: PowerConfig,
    /// Idle time counter (frames)
    idle_frames: u32,
    /// Battery info cache
    battery_info: BatteryInfo,
    /// Last activity time
    last_activity_frame: u32,
    /// Event queue
    events: Vec<PowerEvent>,
}

impl PowerManager {
    /// Create a new power manager
    pub fn new() -> Self {
        PowerManager {
            current_state: PowerState::Active,
            config: PowerConfig::default(),
            idle_frames: 0,
            battery_info: BatteryInfo {
                present: true,
                percentage: 100,
                is_charging: false,
                time_remaining_minutes: 0,
            },
            last_activity_frame: 0,
            events: Vec::new(),
        }
    }

    /// Get current power state
    pub fn current_state(&self) -> PowerState {
        self.current_state
    }

    /// Get battery information
    pub fn battery_info(&self) -> BatteryInfo {
        self.battery_info
    }

    /// Update battery info (called by battery monitor/driver)
    pub fn set_battery_info(&mut self, info: BatteryInfo) {
        self.battery_info = info;
    }

    /// Report user activity — resets idle timer
    pub fn report_activity(&mut self, current_frame: u32) {
        self.last_activity_frame = current_frame;
        self.idle_frames = 0;

        // If in standby/suspend due to idle, wake up
        if self.current_state == PowerState::Standby || self.current_state == PowerState::Suspend {
            self.current_state = PowerState::Active;
            self.events.push(PowerEvent::ActivityDetected);
        }
    }

    /// Tick — called once per frame to update power state
    pub fn tick(&mut self, current_frame: u32, frame_time_ms: u16) {
        // Calculate idle time
        let frames_since_activity = current_frame.wrapping_sub(self.last_activity_frame);
        let seconds_idle = (frames_since_activity as u32 * frame_time_ms as u32) / 1000;

        // Check idle thresholds
        match self.current_state {
            PowerState::Active => {
                if seconds_idle >= self.config.idle_standby_timeout as u32 {
                    self.current_state = PowerState::Standby;
                    self.events.push(PowerEvent::IdleDetected);
                }
            }
            PowerState::Standby => {
                if seconds_idle >= self.config.idle_suspend_timeout as u32 {
                    self.current_state = PowerState::Suspend;
                    self.events.push(PowerEvent::IdleDetected);
                }
            }
            _ => {}
        }

        // Check battery thresholds
        if self.battery_info.present && !self.battery_info.is_charging {
            if self.battery_info.percentage <= self.config.battery_critical_threshold {
                self.events.push(PowerEvent::BatteryCritical);
            } else if self.battery_info.percentage <= self.config.battery_low_threshold {
                self.events.push(PowerEvent::BatteryLow);
            }
        }
    }

    /// Drain pending power events
    pub fn drain_events(&mut self) -> Vec<PowerEvent> {
        let events = self.events.clone();
        self.events.clear();
        events
    }

    /// Request transition to a target power state
    pub fn request_state(&mut self, target: PowerState) -> bool {
        match (self.current_state, target) {
            // Active can transition to Standby, Suspend, Shutdown, Restart
            (PowerState::Active, PowerState::Standby)
            | (PowerState::Active, PowerState::Suspend)
            | (PowerState::Active, PowerState::Shutdown)
            | (PowerState::Active, PowerState::Restart)
            // Standby can transition back to Active or to Suspend, Shutdown, Restart
            | (PowerState::Standby, PowerState::Active)
            | (PowerState::Standby, PowerState::Suspend)
            | (PowerState::Standby, PowerState::Shutdown)
            | (PowerState::Standby, PowerState::Restart)
            // Suspend can transition back to Active or to Shutdown, Restart
            | (PowerState::Suspend, PowerState::Active)
            | (PowerState::Suspend, PowerState::Shutdown)
            | (PowerState::Suspend, PowerState::Restart) => {
                self.current_state = target;
                true
            }
            _ => false,
        }
    }

    /// Enable/disable screen blanking
    pub fn set_screen_blanking(&mut self, enabled: bool) {
        self.config.blank_screen_on_idle = enabled;
    }

    /// Set idle timeout thresholds (in seconds)
    pub fn set_idle_timeouts(&mut self, standby_secs: u16, suspend_secs: u16) {
        self.config.idle_standby_timeout = standby_secs;
        self.config.idle_suspend_timeout = suspend_secs;
    }

    /// Check if screen should be blanked
    pub fn should_blank_screen(&self) -> bool {
        self.config.blank_screen_on_idle && self.current_state == PowerState::Standby
    }

    /// Logout current session
    pub fn logout(&mut self) {
        self.request_state(PowerState::Active);
        self.events.push(PowerEvent::ActivityDetected);
    }

    /// Shutdown the system
    pub fn shutdown(&mut self) {
        self.request_state(PowerState::Shutdown);
    }

    /// Restart the system
    pub fn restart(&mut self) {
        self.request_state(PowerState::Restart);
    }

    /// Get human-readable state name
    pub fn state_name(&self) -> &'static str {
        match self.current_state {
            PowerState::Active => "Active",
            PowerState::Idle => "Idle",
            PowerState::Standby => "Standby",
            PowerState::Suspend => "Suspend",
            PowerState::Hibernate => "Hibernate",
            PowerState::Shutdown => "Shutdown",
            PowerState::Restart => "Restart",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_manager_creation() {
        let pm = PowerManager::new();
        assert_eq!(pm.current_state(), PowerState::Active);
        assert!(pm.battery_info().present);
    }

    #[test]
    fn test_activity_resets_idle() {
        let mut pm = PowerManager::new();
        pm.report_activity(100);
        assert_eq!(pm.last_activity_frame, 100);
    }

    #[test]
    fn test_state_transitions() {
        let mut pm = PowerManager::new();
        assert!(pm.request_state(PowerState::Standby));
        assert_eq!(pm.current_state(), PowerState::Standby);
        assert!(pm.request_state(PowerState::Active));
        assert_eq!(pm.current_state(), PowerState::Active);
    }

    #[test]
    fn test_battery_thresholds() {
        let mut pm = PowerManager::new();
        pm.set_battery_info(BatteryInfo {
            present: true,
            percentage: 5,
            is_charging: false,
            time_remaining_minutes: 10,
        });
        pm.tick(1000, 16);
        let events = pm.drain_events();
        assert!(events.iter().any(|e| *e == PowerEvent::BatteryCritical));
    }
}
