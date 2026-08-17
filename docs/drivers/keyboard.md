# Keyboard Driver

The keyboard driver handles scancode input and distribution to the shell and GUI.

## Scancode Capture

PS/2 IRQ1 pushes raw scancodes into the driver. The entry point is `kernel/kernel/src/keyboard.rs` (`handle_scancode`), which forwards to `task/keyboard.rs`.

## Scancode Queues (`task/keyboard.rs`)

```rust
pub fn add_scancode(scancode: u8);        // push to both queues, wake the stream
pub fn try_pop_scancode() -> Option<u8>;  // non-blocking pop for the GUI compositor
pub struct ScancodeStream;                // async Stream<Item = u8> for the shell
```

Every scancode is pushed to **two** queues:
- `SCANCODE_QUEUE` — consumed by the shell via the async `ScancodeStream` (registered with a `Waker` so the executor wakes on input)
- `GUI_SCANCODE_QUEUE` — polled non-blockingly by the GUI/compositor via `try_pop_scancode`

If the shell queue is full, the scancode is dropped with a warning.

## Scancode Decoding

Decoding to keys is done with the `pc_keyboard` crate by consumers (the compositor and window input handler), e.g. `pc_keyboard::DecodedKey::RawKey(...)` / `Unicode(...)`. Modifier keys (alt, etc.) are tracked by the compositor to enable shortcuts (e.g. Alt+F4).

## Scancode Sets

Scancodes are the standard PS/2 set-1 make/break codes produced by the 8042 keyboard device (break code = 0x80 + make code). No in-kernel keymap table exists; decoding is deferred to `pc_keyboard`.

## LED / Typematic

No LED synchronization or typematic configuration is performed by the driver; the 8042 self-test/defaults are set during `ps2::init()` (see `docs/drivers/ps2.md`).
