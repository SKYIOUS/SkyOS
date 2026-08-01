# Framebuffer Graphics Driver

The graphics subsystem provides display output through the BGA (Bochs) virtual VGA and a software console.

## BGA (Bochs Graphics Adapter) — `drivers/graphics/bga.rs`

The `Bga` struct drives the Bochs VBE interface through two I/O ports (0x01CE index, 0x01CF data):

- VBE_DISPI_INDEX_ID / XRES / YRES / BPP / ENABLE / BANK / VIRT_WIDTH / VIRT_HEIGHT / X_OFFSET / Y_OFFSET
- `VBE_DISPI_ENABLED | VBE_DISPI_LFB_ENABLED` enables the linear framebuffer

## Console — `drivers/graphics/console.rs`

`ConsoleWriter` implements `core::fmt::Write` and renders text to the framebuffer with ANSI escape handling:

```rust
pub struct ConsoleWriter;                    // fmt::Write + escape processing
pub fn _print(args: fmt::Arguments);         // kernel print entry (println!/print!)
pub fn set_console_color(fg: u32, bg: u32);
```

It supports clear screen (`clear_screen`), scrollback, and escape sequences (e.g. `\x1b[2J`). The font is rendered from a PSF font (`drivers/graphics/psf.rs`).

## Frame Buffer

The kernel receives the framebuffer from the bootloader (UEFI GOP framebuffer address/size/stride). There is no `Framebuffer` struct with drawing primitives as described in older docs; drawing happens through the BGA/console layer and the compositor (`kernel/kernel/src/compositor/`).

## Future GPU Support

The `drivers/gpu/` module contains a VirtIO GPU driver (`virtio_gpu.rs`) with a ring-based command interface (`ring.rs`). See `docs/design/gui_architecture.md` for the compositor architecture.
