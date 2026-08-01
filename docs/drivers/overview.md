# Driver Subsystem Overview

The SkyOS driver subsystem is organized by device class. Drivers are built into the kernel; there
is no loadable-module mechanism. Storage devices implement the `BlockDevice` trait, character
devices integrate into the VFS via `VfsNode`, and a kext framework (`kernel/kernel/src/kext/`) provides a
Nub/Family registry for PCI/USB/platform devices (see `docs/DRIVER_MODEL.md` and
`docs/api/driver_api.md`).

## Driver Lifecycle

1. **Discovery**: PCI bus enumeration finds devices (via the `pci/` module) and creates device
   nubs (`PciDeviceNub`, `UsbDeviceNub`, `PlatformDeviceNub`)
2. **Matching**: `DriverFamily` implementations (`NetFamily`, `StorageFamily`, `GraphicsFamily`)
   match nubs to driver logic
3. **Initialization**: `drivers::init()` boots serial and RTC; each subsystem (storage, net,
   graphics, gpu, usb, input, audio) initializes the hardware it finds
4. **Operation**: Block devices serve VFS filesystem requests; char devices handle reads/writes
5. **Shutdown**: `drivers::cleanup()` (serial/RTC) and subsystem cleanup paths

## Current Drivers

| Driver | Module | Status |
|--------|--------|--------|
| PS/2 Controller / Keyboard / Mouse | `drivers/ps2.rs`, `drivers/mouse.rs` | Done |
| Framebuffer / Console | `drivers/graphics/bga.rs`, `drivers/graphics/console.rs` | Done |
| PC Speaker | `drivers/audio/pcspeaker.rs` | Done |
| Real-Time Clock | `drivers/rtc.rs` | Done |
| Serial | `drivers/serial.rs` | Done |
| PATA | `drivers/storage/pata.rs` | Done |
| AHCI | `drivers/storage/ahci.rs` | Implemented |
| NVMe | `drivers/storage/nvme.rs` | Implemented |
| VirtIO Block | `drivers/storage/virtio_block.rs` | Implemented |
| VirtIO GPU | `drivers/gpu/virtio_gpu.rs` | Implemented |
| Intel e1000 | `drivers/net/e1000.rs` | Implemented (`net` feature) |
| VirtIO Net | `drivers/net/virtio.rs` | Implemented (`net` feature) |
| USB (UHCI/xHCI) | `drivers/usb/uhci.rs`, `drivers/usb/xhci.rs` | Implemented |
| HDA Audio | `drivers/audio/hda.rs` | Implemented |
| Watchdog | `drivers/watchdog.rs` | Implemented |
| ACPI | `kernel/kernel/src/acpi.rs` (RSDP/MADT, IOAPIC + LAPIC info) | Implemented |

## Driver Interface

There is no single `DeviceDriver` trait. The three interfaces are:

```rust
// Block devices (drivers/block/mod.rs)
pub trait BlockDevice: Send + Sync {
    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> Result<(), BlockDeviceError>;
    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<(), BlockDeviceError>;
    fn sector_count(&self) -> Result<u64, BlockDeviceError>;
    fn sync(&mut self) {}
}

// Kext nubs (kext/nub.rs)
pub trait Nub: Send + Sync {
    fn nub_name(&self) -> &'static str;
    fn kind(&self) -> NubKind;
    fn match_driver(&self, driver_name: &str) -> bool;
    fn start(&self) -> bool;
    fn stop(&self);
}

// Driver families (kext/family.rs)
pub trait DriverFamily: Send + Sync {
    fn family_name(&self) -> &'static str;
    fn match_nub(&self, nub: &Arc<dyn Nub>) -> bool;
    fn start_driver(&self, nub: Arc<dyn Nub>) -> Result<(), ()>;
    fn stop_driver(&self, nub: &Arc<dyn Nub>);
}
```

Drivers communicate with hardware through MMIO regions, I/O ports, and DMA. Interrupt handlers are
registered with the kernel's IRQ system.
