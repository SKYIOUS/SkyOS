# Mouse Driver

The PS/2 mouse driver (`kernel/kernel/src/drivers/mouse.rs`) handles relative motion input from standard PS/2 mice, fed by IRQ12.

## Public API

```rust
pub fn init();              // called after ps2::init() enables the mouse
pub fn enable_wheel();      // armed by ps2::init() when the IntelliMouse device ID is 3 or 4
pub fn feed_byte(byte: u8); // push a packet byte into the decoder
pub fn handle_interrupt();  // IRQ12 handler: reads port 0x60, feeds feed_byte
```

There is no `MouseDevice` struct or `DriverError`.

## Data Packet Format

Standard PS/2 mouse uses 3-byte packets:

| Byte | Bits | Description |
|------|------|-------------|
| 0 | 0 | Left button |
| 0 | 1 | Right button |
| 0 | 2 | Middle button |
| 0 | 4 | X sign (9th bit) |
| 0 | 5 | Y sign (9th bit) |
| 0 | 6 | X overflow |
| 0 | 7 | Y overflow |
| 1 | 0-7 | X movement delta |
| 2 | 0-7 | Y movement delta |

IntelliMouse protocol adds a 4th byte for scroll wheel data (enabled via `enable_wheel`).

## Event Processing

`feed_byte` accumulates packet bytes and converts them into motion events:
- Accumulates delta X/Y values
- Tracks button state (press/release)
- Reports scroll wheel movement
- Handles overflow conditions by discarding large deltas

Processed events feed the input layer (`drivers/input.rs`) and ultimately the GUI/compositor.
