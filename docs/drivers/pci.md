# PCI Device Access

The PCI subsystem (`kernel/kernel/src/pci/mod.rs`) provides configuration-space access and device enumeration via the legacy I/O port method (0xCF8 config address, 0xCFC config data). There is no ECAM/MMIO access, and no `PciConfig`/`PciDevice` structs — accesses are raw read/write helpers taking `(bus, slot, func)`.

## Configuration Access

```rust
pub fn read_config_u32(bus: u8, slot: u8, func: u8, offset: u8) -> u32;
pub fn read_config_u16(bus: u8, slot: u8, func: u8, offset: u8) -> u16;
pub fn read_config_u8(bus: u8, slot: u8, func: u8, offset: u8) -> u8;
pub fn write_config_u32(bus: u8, slot: u8, func: u8, offset: u8, value: u32);
pub fn write_config_u16(bus: u8, slot: u8, func: u8, offset: u8, value: u16);
```

The 32-bit accessor builds the standard PCI config address word (`enable | bus<<16 | slot<<11 | func<<8 | offset&0xFC`); the u16/u8 variants shift the result. `read_bar64(bus, slot, func, bar_offset)` reads a 64-bit BAR (detecting it via the low bits and reading the upper dword).

## Enumeration

`pub fn enumerate_pci()` scans the bus hierarchy (recursively handling PCI-to-PCI bridges), using `vendor_id == 0xFFFF` as "no device". Enumeration calls the driver hooks for discovered devices.

## Capabilities & MSI

- `find_capability(bus, slot, func, cap_id) -> Option<u8>` walks the capability list from the status-register capabilities bit + `0x34` pointer.
- `pci_enable_msi(bus, slot, func) -> Option<u8>` finds the MSI capability, allocates a vector from `apic::msi`, writes the MSI address/data registers, and enables MSI (single-message, MME=0).
- Legacy interrupts are routed through the I/O APIC via `apic::route_pci_irq`.

## Base Address Registers (BARs)

BARs are read via `read_bar64` (handles both 32-bit and 64-bit memory BARs, plus I/O BARs). The raw value is mapped to a virtual address via the physical-memory offset (`bar_to_virt`).
