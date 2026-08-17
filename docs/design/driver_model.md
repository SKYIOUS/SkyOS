# Driver Model and Framework

The SkyOS driver framework (`kernel/kernel/src/drivers/`) provides hardware device support. Drivers are compiled into the kernel; there is no loadable-module mechanism.

## Driver Interfaces

There is no single `DeviceDriver` trait. Hardware support is split across three interfaces:

1. **`BlockDevice`** (`drivers/block/mod.rs`) — sector-addressable storage (`read_sector`/`write_sector`/`sector_count`). Registered via `register_block_device`, which wraps the device in a `BlockCache`. Mounted by VFS filesystems.
2. **`VfsNode`** (`vfs/mod.rs`) — character/device nodes integrated into the VFS tree (framebuffer, input, etc.).
3. **Kext framework** (`kext/`) — a Nub/Family registry for PCI/USB/platform device discovery (see `docs/api/driver_api.md`).

## Driver Lifecycle

1. **Discovery**: PCI bus enumeration (`pci/`) finds devices and creates nubs (`PciDeviceNub`, `UsbDeviceNub`, `PlatformDeviceNub`). ACPI provides interrupt routing and power info (`kernel/kernel/src/acpi.rs`).
2. **Matching**: `DriverFamily` implementations (`NetFamily`, `StorageFamily`, `GraphicsFamily`) match nubs by PCI class code.
3. **Initialization**: `drivers::init()` boots serial and RTC; each subsystem (storage, net, graphics, gpu, usb, input, audio) initializes what it finds.
4. **Operation**: Block devices serve VFS filesystem requests; char devices handle reads/writes; net drivers adapt to smoltcp's `Device` trait.
5. **Shutdown**: `drivers::cleanup()` (serial/RTC) plus subsystem cleanup.

## Device Discovery

PCI devices are found via legacy configuration-port enumeration (`pci::enumerate_pci`), which walks buses and slots (recursively for bridges). Each function is probed; `vendor_id == 0xFFFF` means "absent". The kext framework turns discovered functions into nubs.

## DMA Support

There is no centralized DMA API. Drivers access device memory through the physical-memory offset (`memory::physical_memory_offset()`) and I/O ports; block/network drivers manage their own buffers (e.g. the VirtIO queue rings in `drivers/net/virtio.rs`). No IOMMU/scatter-gather abstraction exists.
