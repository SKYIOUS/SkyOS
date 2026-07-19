//! Network Framework — Ethernet, Wi-Fi abstraction, VPN backend, connection manager.
//!
//! Manages network devices, connections, and internet services.
//! Provides abstraction for future hardware driver integration.

use alloc::vec::Vec;
use alloc::string::String;

/// Network device type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkDeviceType {
    Ethernet,
    WiFi,
    Cellular,
    VPN,
}

/// Network connection state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting...
    Connecting,
    /// Connected
    Connected,
    /// Authentication in progress
    Authenticating,
    /// Obtaining IP address
    ConfiguringIp,
    /// Failed
    Failed,
}

/// IP version
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpVersion {
    IPv4,
    IPv6,
}

/// IP address information
#[derive(Clone)]
pub struct IpInfo {
    pub version: IpVersion,
    pub address: String,
    pub gateway: String,
    pub dns_primary: String,
    pub dns_secondary: String,
}

/// WiFi network information
#[derive(Clone)]
pub struct WiFiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub strength: u8,     // 0-100
    pub frequency: u16,   // MHz
    pub is_secured: bool,
}

/// Network device
#[derive(Clone)]
pub struct NetworkDevice {
    pub id: u32,
    pub name: String,
    pub device_type: NetworkDeviceType,
    pub state: ConnectionState,
    pub mac_address: String,
    pub speed_mbps: u32,
    pub ip_info: Option<IpInfo>,
}

/// Network connection
#[derive(Clone)]
pub struct NetworkConnection {
    pub id: u32,
    pub name: String,
    pub device_id: u32,
    pub state: ConnectionState,
    pub is_active: bool,
}

/// Network event type
#[derive(Clone, Copy, Debug)]
pub enum NetworkEvent {
    /// Device connected/appeared
    DeviceAdded(u32),
    /// Device disconnected/removed
    DeviceRemoved(u32),
    /// Connection state changed
    StateChanged(u32, ConnectionState),
    /// IP address obtained
    IpAcquired(u32),
    /// IP address lost
    IpLost(u32),
    /// Signal strength changed
    SignalChanged(u32, u8),
}

/// Network Manager
pub struct NetworkManager {
    /// Network devices
    devices: Vec<NetworkDevice>,
    /// Active connections
    connections: Vec<NetworkConnection>,
    /// Available WiFi networks
    wifi_networks: Vec<WiFiNetwork>,
    /// Pending events
    events: Vec<NetworkEvent>,
}

impl NetworkManager {
    /// Create a new network manager
    pub fn new() -> Self {
        let mut manager = NetworkManager {
            devices: Vec::new(),
            connections: Vec::new(),
            wifi_networks: Vec::new(),
            events: Vec::new(),
        };

        // Register default Ethernet device
        manager.devices.push(NetworkDevice {
            id: 0,
            name: String::from("eth0"),
            device_type: NetworkDeviceType::Ethernet,
            state: ConnectionState::Disconnected,
            mac_address: String::from("00:00:00:00:00:00"),
            speed_mbps: 0,
            ip_info: None,
        });

        manager
    }

    /// Get device by ID
    pub fn get_device(&self, id: u32) -> Option<&NetworkDevice> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// Get mutable device
    pub fn get_device_mut(&mut self, id: u32) -> Option<&mut NetworkDevice> {
        self.devices.iter_mut().find(|d| d.id == id)
    }

    /// Get all devices
    pub fn devices(&self) -> &[NetworkDevice] {
        &self.devices
    }

    /// Register a new network device
    pub fn register_device(
        &mut self,
        name: &str,
        device_type: NetworkDeviceType,
        mac_address: &str,
    ) -> u32 {
        let id = self.devices.iter().map(|d| d.id).max().unwrap_or(0) + 1;

        self.devices.push(NetworkDevice {
            id,
            name: String::from(name),
            device_type,
            state: ConnectionState::Disconnected,
            mac_address: String::from(mac_address),
            speed_mbps: 0,
            ip_info: None,
        });

        self.events.push(NetworkEvent::DeviceAdded(id));
        id
    }

    /// Unregister a network device
    pub fn unregister_device(&mut self, id: u32) {
        self.devices.retain(|d| d.id != id);
        self.connections.retain(|c| c.device_id != id);
        self.events.push(NetworkEvent::DeviceRemoved(id));
    }

    /// Update device connection state
    pub fn set_device_state(&mut self, id: u32, state: ConnectionState) -> bool {
        if let Some(device) = self.get_device_mut(id) {
            let old_state = device.state;
            device.state = state;
            if old_state != state {
                self.events.push(NetworkEvent::StateChanged(id, state));
            }
            true
        } else {
            false
        }
    }

    /// Set device IP information
    pub fn set_ip_info(&mut self, id: u32, ip_info: IpInfo) -> bool {
        if let Some(device) = self.get_device_mut(id) {
            device.ip_info = Some(ip_info);
            device.state = ConnectionState::Connected;
            self.events.push(NetworkEvent::IpAcquired(id));
            true
        } else {
            false
        }
    }

    /// Clear device IP information
    pub fn clear_ip_info(&mut self, id: u32) -> bool {
        if let Some(device) = self.get_device_mut(id) {
            device.ip_info = None;
            self.events.push(NetworkEvent::IpLost(id));
            true
        } else {
            false
        }
    }

    /// Set device speed
    pub fn set_speed(&mut self, id: u32, speed_mbps: u32) -> bool {
        if let Some(device) = self.get_device_mut(id) {
            device.speed_mbps = speed_mbps;
            true
        } else {
            false
        }
    }

    /// Scan for WiFi networks
    pub fn scan_wifi(&mut self, device_id: u32) -> bool {
        if let Some(device) = self.get_device(device_id) {
            if device.device_type == NetworkDeviceType::WiFi {
                self.set_device_state(device_id, ConnectionState::Connecting);
                return true;
            }
        }
        false
    }

    /// Add WiFi network (from scan results)
    pub fn add_wifi_network(&mut self, network: WiFiNetwork) {
        if !self.wifi_networks.iter().any(|n| n.ssid == network.ssid) {
            self.wifi_networks.push(network);
        }
    }

    /// Get WiFi networks
    pub fn wifi_networks(&self) -> &[WiFiNetwork] {
        &self.wifi_networks
    }

    /// Clear WiFi networks
    pub fn clear_wifi_networks(&mut self) {
        self.wifi_networks.clear();
    }

    /// Connect to a network
    pub fn connect(&mut self, device_id: u32, connection_name: &str) -> u32 {
        let conn_id = self.connections.iter().map(|c| c.id).max().unwrap_or(0) + 1;

        self.connections.push(NetworkConnection {
            id: conn_id,
            name: String::from(connection_name),
            device_id,
            state: ConnectionState::Connecting,
            is_active: false,
        });

        if let Some(device) = self.get_device_mut(device_id) {
            device.state = ConnectionState::Authenticating;
        }

        conn_id
    }

    /// Disconnect from network
    pub fn disconnect(&mut self, device_id: u32) -> bool {
        if let Some(device) = self.get_device_mut(device_id) {
            device.state = ConnectionState::Disconnected;
            self.connections.retain(|c| c.device_id != device_id);
            self.events.push(NetworkEvent::StateChanged(device_id, ConnectionState::Disconnected));
            true
        } else {
            false
        }
    }

    /// Get connection state
    pub fn connection_state(&self, device_id: u32) -> Option<ConnectionState> {
        self.get_device(device_id).map(|d| d.state)
    }

    /// Check if connected to network
    pub fn is_connected(&self) -> bool {
        self.devices
            .iter()
            .any(|d| d.state == ConnectionState::Connected && d.ip_info.is_some())
    }

    /// Get default connected device
    pub fn default_connection(&self) -> Option<&NetworkDevice> {
        self.devices
            .iter()
            .find(|d| d.state == ConnectionState::Connected && d.ip_info.is_some())
    }

    /// Update WiFi signal strength
    pub fn set_signal_strength(&mut self, device_id: u32, strength: u8) {
        if let Some(device) = self.get_device_mut(device_id) {
            if device.device_type == NetworkDeviceType::WiFi {
                self.events.push(NetworkEvent::SignalChanged(device_id, strength));
            }
        }
    }

    /// Drain pending network events
    pub fn drain_events(&mut self) -> Vec<NetworkEvent> {
        let events = self.events.clone();
        self.events.clear();
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_manager_creation() {
        let nm = NetworkManager::new();
        assert!(!nm.devices().is_empty());
    }

    #[test]
    fn test_register_device() {
        let mut nm = NetworkManager::new();
        let id = nm.register_device("wlan0", NetworkDeviceType::WiFi, "00:11:22:33:44:55");
        assert!(nm.get_device(id).is_some());
    }

    #[test]
    fn test_set_connection_state() {
        let mut nm = NetworkManager::new();
        assert!(nm.set_device_state(0, ConnectionState::Connected));
        let device = nm.get_device(0).unwrap();
        assert_eq!(device.state, ConnectionState::Connected);
    }

    #[test]
    fn test_is_connected() {
        let mut nm = NetworkManager::new();
        nm.set_device_state(0, ConnectionState::Connected);
        nm.set_ip_info(
            0,
            IpInfo {
                version: IpVersion::IPv4,
                address: String::from("192.168.1.100"),
                gateway: String::from("192.168.1.1"),
                dns_primary: String::from("8.8.8.8"),
                dns_secondary: String::from("8.8.4.4"),
            },
        );
        assert!(nm.is_connected());
    }
}
