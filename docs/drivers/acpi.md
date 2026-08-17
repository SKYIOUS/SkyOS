# ACPI Tables and Configuration

The ACPI subsystem (`kernel/kernel/src/acpi.rs`) provides hardware configuration and power management, built on the `acpi` crate.

## Implementation

`SkyAcpiHandler` implements the `acpi::AcpiHandler` trait, mapping physical regions through the kernel's physical-memory offset. `acpi::init(boot_rsdp)`:

1. Locates the RSDP (from the bootloader-provided `boot_rsdp`, or a firmware search of the EBDA and the `0xE0000..0x100000` BIOS area)
2. Loads tables via `AcpiTables::from_rsdp`
3. Parses `platform_info()`:
   - **Local APIC** address → `LAPIC_ADDR`
   - **I/O APIC** addresses → `IOAPIC_ADDRS`
   - **Interrupt source overrides** → `OVERRIDES` (ISA IRQ, GSI, polarity, trigger)
   - **Application processors** → `AP_LAPIC_IDS` (APIC IDs of cores to bring up under SMP)
4. Parses the **FADT** for power management (`parse_fadt`):
   - PM1a/PM1b control-block ports → `PM1A_CNT_PORT` / `PM1B_CNT_PORT`
   - Reset register (port, value) → `RESET_REG_PORT`
   - 8042 PS/2 presence flag → `PS2_PRESENT`

There is no `AcpiTables` struct in kernel code (the `acpi` crate provides it), no AML interpreter, and no `SleepState`/`DriverError` types.

## Power Management

```rust
pub fn acpi_shutdown();  // S5 soft-off via PM1a/PM1b (SLP_TYP + SLP_EN), then halt
pub fn acpi_reboot();    // system reset via RESET_REG, falling back to the 8042 reset
```

- `acpi_shutdown()` writes `SLP_TYP_S5 | SLP_EN (0x2000)` to the PM1a port (and PM1b if present), then disables interrupts and halts. The S5 `SLP_TYP` value is currently hardcoded to 0 as a best-effort default rather than parsed from `\_S5` in the DSDT.
- `acpi_reboot()` writes the FADT reset value to the reset register; if no reset register exists it falls back to the keyboard-controller reset command (0xFE on port 0x64).
