# Driver Interface API

SkyOS does not use a single `DeviceDriver` trait. Hardware support is split across three
mechanisms (see `docs/DRIVER_MODEL.md` and `docs/design/driver_model.md`).

## Block Devices

Storage drivers (AHCI/SATA, PATA, VirtIO-Block, NVMe) implement the `BlockDevice` trait:

```rust
pub trait BlockDevice: Send + Sync {
    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> Result<(), BlockDeviceError>;
    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<(), BlockDeviceError>;
    fn sector_count(&self) -> Result<u64, BlockDeviceError>;
    fn sync(&mut self) -> Result<(), BlockDeviceError> { ... }
}
```

Defined in `kernel/src/drivers/block/mod.rs`. Filesystems are mounted on top of these.

## Character Devices

Character devices (serial, console, input) are integrated into the VFS and implement the `VfsNode`
trait (see `docs/api/vfs_api.md`). They are exposed under `/dev`.

## Kext Framework

The `kext/` module provides a structured, extensible driver registry:

```rust
// kernel/src/kext/nub.rs
pub enum NubKind { Pci, Usb, Platform }

pub trait Nub: Send + Sync {
    fn kind(&self) -> NubKind;
    // ...
}

pub struct PciDeviceNub { ... }     // vendor_id, device_id, class_code, bus, slot, function, irq
pub struct UsbDeviceNub { ... }     // vendor_id, product_id, class_code, subclass, protocol
pub struct PlatformDeviceNub { ... } // name, compatible
```

```rust
// kernel/src/kext/family.rs
pub trait DriverFamily: Send + Sync {
    fn name(&self) -> &'static str;
    fn matches(&self, nub: &dyn Nub) -> bool;
    // ...
}
pub struct NetFamily;       // NIC drivers
pub struct StorageFamily;   // block devices
pub struct GraphicsFamily;  // GPUs
```

```rust
// kernel/src/kext/loader.rs
pub fn register_nub(nub: Arc<dyn Nub>);
pub fn register_family(family: Arc<dyn DriverFamily>);
pub fn init();

// kernel/src/kext/mod.rs
pub fn register_kext(name: &str, version: (u16, u16, u16), vendor: &str, description: &str) -> KextId;
pub fn get_kext(id: KextId) -> Option<KextInfo>;
pub fn list_kexts() -> Vec<KextInfo>;
```

`kext/isolation.rs` provides a `DriverObject` wrapper with crash detection and restart support.

## Driver Lifecycle

Drivers are built into the kernel binary; there are no loadable modules. `drivers::init()` starts
the serial and RTC drivers, and individual subsystems (storage, net, graphics, usb, input, audio)
initialize the devices they find during PCI enumeration.
