# Interrupt Handling Architecture

SkyOS uses the x86 APIC architecture for interrupt management. The local APIC runs in ExtINT mode (pass-through) so the legacy PIC owns the interrupt lifecycle; hardware IRQs are forwarded from PIC to LAPIC.

## Interrupt Descriptor Table

The IDT is set up during boot with entries for all 256 interrupt vectors. Each entry specifies a handler function, privilege level, and (for the double-fault vector) an interrupt stack table (IST) index.

```rust
#[repr(C, packed)]
pub struct IdtEntry {
    handler_low: u16,
    gdt_selector: u16,
    options: IdtOptions,
    handler_mid: u16,
    handler_high: u32,
    reserved: u32,
}
```

## APIC and I/O APIC

The local APIC handles CPU-local interrupts (LAPIC timer, IPIs); the I/O APIC distributes hardware IRQs from peripherals. MSI support exists in `apic/msi.rs`; MSI-X is not used.

## IRQ Handling Flow

1. Hardware interrupt fires → I/O APIC delivers to target CPU's local APIC
2. CPU saves registers and enters interrupt handler via IDT
3. Handler acknowledges the interrupt (EOI)
4. Driver-specific processing runs (e.g., E1000 receive at interrupts.rs:413)
5. Return from interrupt restores saved registers

## Spurious Interrupts

The kernel handles spurious interrupts by masking the corresponding IRQ line and logging the event. Spurious interrupts typically indicate electrical noise or hardware glitches on the interrupt line.

## Interrupt Safety

Only the **double-fault handler runs on a dedicated IST stack** (`DOUBLE_FAULT_IST_INDEX`); other handlers use the interrupted task's stack. Locks within interrupt handlers use spinlocks with interrupt disabling to prevent deadlocks.
