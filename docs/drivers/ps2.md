# PS/2 Controller Driver

The PS/2 controller driver (`kernel/kernel/src/drivers/ps2.rs`) manages the legacy 8042 interface for keyboard and mouse input.

## Initialization

`ps2::init()` is called during boot and performs the full 8042 sequence:

1. Lock the controller (`PS2_LOCK` spinlock)
2. Disable both ports (command 0xAD = disable keyboard, 0xA7 = disable mouse)
3. Flush the output buffer
4. Read the config byte (0x20), set bits 0–1 (enable keyboard + mouse IRQs), write it back (0x60)
5. Controller self-test (0xAA, expect 0x55), then re-write the config byte (some 8042s reset it during self-test)
6. Enable both devices (0xAE keyboard, 0xA8 mouse)
7. Reset + set defaults + enable scanning on the keyboard (0xFF/0xF6/0xF4)
8. Reset + set defaults on the mouse, run the IntelliMouse scroll-wheel detection sequence (sample rates 200/100/80), read the device ID — 3 or 4 means a wheel is present, which arms `mouse::enable_wheel()`
9. Enable streaming (0xF4), flush stale bytes
10. Unmask keyboard IRQ1 and mouse IRQ12 on the legacy PICs (via I/O ports 0x21/0xA1 — IOAPIC delivery is currently bypassed)

The public surface is just `pub fn init()`; the rest is module-private. There is no `Ps2Controller` struct or `DriverError`.

## Port Access

The controller uses I/O ports 0x60 (data) and 0x64 (command/status), with `wait_write`/`wait_read` polling loops on the status register.

## Interrupts

Port 1 (keyboard) uses IRQ 1, port 2 (mouse) uses IRQ 12. Interrupts are delivered through the legacy PIC; the mouse handler is in `drivers/mouse.rs` (`handle_interrupt`).

## Device Access

`device_write_to_keyboard(data)` writes data then reads the ACK. `device_write_to_mouse(data)` first issues the 0xD4 "write to aux device" command to the controller.
