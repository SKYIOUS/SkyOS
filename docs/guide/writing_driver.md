# Writing a Kernel Driver

This guide explains how to write a driver for a hardware device in SkyOS.

## Driver Model

There is no single `DeviceDriver` trait. Two mechanisms exist:

1. **Block/storage devices** — implement the `BlockDevice` trait and register via `register_block_device` (see `kernel/kernel/src/drivers/block/mod.rs`).
2. **Device discovery (kext framework)** — PCI/network/storage/graphics devices are represented as **nubs** and matched to **driver families** (see `kernel/kernel/src/kext/`).

## Step 1: Block Device Driver

Create a module under `kernel/kernel/src/drivers/storage/` and implement `BlockDevice`:

```rust
use crate::drivers::block::{BlockDevice, BlockDeviceError};

pub struct MyDisk { /* sector count, backing storage */ }

impl BlockDevice for MyDisk {
    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> Result<(), BlockDeviceError> {
        // copy sector `sector` into `buf`
        Ok(())
    }
    fn write_sector(&mut self, sector: u64, buf: &[u8]) -> Result<(), BlockDeviceError> {
        // store `buf` into sector `sector`
        Ok(())
    }
    fn sector_count(&self) -> Result<u64, BlockDeviceError> {
        Ok(/* total sectors */)
    }
}
```

## Step 2: Register the Device

Registration wraps the device in a `BlockCache` and pushes it onto the global device list:

```rust
use alloc::sync::Arc;
use spin::Mutex;

let disk = Arc::new(Mutex::new(MyDisk { /* ... */ }));
register_block_device(disk);  // block cache + registration
```

Consumers mount filesystems from the registered device via `BLOCK_DEVICES` or by index.

## The kext Nub/Family Model

For hot-plug/discovery-oriented device classes, the kext framework (`kernel/kernel/src/kext/`) models devices as nubs:

- **Nub** (`nub.rs`): a point of connection — `PciDeviceNub`, `UsbDeviceNub`, `PlatformDeviceNub`. A PCI nub exposes `vendor_id`, `device_id`, `class_code`, `subclass`, `bus`, `device`, `function`, `irq`, and a `match_driver("pci:ven=...,dev=...,class=...")` matcher.
- **DriverFamily** (`family.rs`): groups nubs by function and starts the matching driver — `NetFamily` (class 0x02), `StorageFamily` (class 0x01), `GraphicsFamily` (class 0x03).

A family matches a nub and drives it:

```rust
impl DriverFamily for NetFamily {
    fn family_name(&self) -> &'static str { "Network" }
    fn match_nub(&self, nub: &Arc<dyn Nub>) -> bool {
        matches!(nub.kind(), NubKind::Pci(p) if p.class_code == 0x02)
    }
    fn start_driver(&self, nub: Arc<dyn Nub>) -> Result<(), ()> {
        // probe + initialize the device, register its services
        Ok(())
    }
    fn stop_driver(&self, _nub: &Arc<dyn Nub>) {}
}
```

Register a family/kext with the kext manager (`kext/mod.rs`):

```rust
register_kext("mykext", (0, 1, 0), "vendor", "description"); // returns KextId
```

## Interrupt Handling

Drivers register interrupt handlers via the APIC/IOAPIC layer (see `kernel/kernel/src/apic/`). Handlers run in interrupt context and should be minimal; defer work to the async executor or a kernel thread.
