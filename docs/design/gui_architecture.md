# GUI Compositor Design

The SkyOS GUI subsystem is an **in-kernel compositor** (`kernel/src/compositor/`, window state in
`kernel/src/gui/`). There is no userspace display server; applications talk to the kernel
directly through the GUI syscalls (see `docs/api/gui_syscalls.md`).

## Architecture Overview

- The kernel compositor owns a `Vec<Window>` and a framebuffer covering the whole screen.
- Applications create windows via `gui_create_window` (syscall 100), which allocates a shared
  physical framebuffer (`width * height` 32-bit pixels) or falls back to an in-memory content
  buffer.
- `gui_map_buffer` (103) maps the window framebuffer into the application's address space so it can
  draw directly; `gui_flush` (102) copies the user buffer into the window content and triggers a
  render. Physically-backed windows render without a copy.
- The ADE desktop process is the primary client and layers windows on top of each other.

## Window Buffers

Each window has a backing buffer allocated by the kernel. Pixels are 32-bit; color values are
written as `0xAARRGGBB` (see `gui/window.rs`). The buffer is writable by the application and read
by the compositor.

## Compositing

Rendering is damage-based:

1. Applications flush via `gui_flush` (102)
2. The compositor redraws windows bottom-up (window order, then focused/topmost last)
3. Only the damaged region is re-composited, then blitted to the display framebuffer
4. Decorations (title bar, window chrome) are drawn by the kernel compositor

Helpers live in `compositor/` (`blend.rs`, `blur.rs`, `flush.rs`, `shadow.rs`, `vsync.rs`).

## Input Handling

Input is not pushed through a device file or event queue that blocks. Applications poll
`gui_get_key` (105) and `gui_get_mouse` (120) per frame from their event loop; the ADE desktop
polls at ~60 Hz. The compositor routes focus/cursor state.

## Window Management

All window management is done with kernel syscalls: `gui_set_title` (121), `gui_destroy_window`
(122), `gui_resize_window` (123), `gui_move_window` (124), plus `clipboard` (125) and `notify`
(126).
