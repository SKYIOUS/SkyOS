# GUI System Calls

SkyOS exposes an in-kernel compositor through GUI syscalls in the 100–105 and 120–126 ranges
(see `kernel/src/syscalls/numbers.rs`). Each window is identified by a handle (an index into the
compositor's window list).

| # | Name | Description |
|---|------|-------------|
| 100 | gui_create_window | Create a window (title, width, height) |
| 101 | gui_get_buffer | Get window content size (packed width/height) |
| 102 | gui_flush | Copy user buffer into window and render |
| 103 | gui_map_buffer | Map the window framebuffer into user space |
| 104 | beep | Emit a PC-speaker tone |
| 105 | gui_get_key | Pop the next queued key event |
| 120 | gui_get_mouse | Get mouse state relative to content area |
| 121 | gui_set_title | Update the window title |
| 122 | gui_destroy_window | Remove the window from the compositor |
| 123 | gui_resize_window | Resize the window |
| 124 | gui_move_window | Move the window |
| 125 | clipboard | Read/write the compositor clipboard |
| 126 | notify | Queue a desktop notification |

Userspace applications normally call these through `libsarga::gui::Window` rather than issuing the
syscalls directly. Full signatures, packed return formats, and the event-handling model are
documented in `docs/api/gui_syscalls.md`.

There is no separate "display info", "set cursor", or "event wait" syscall; input is polled per
frame via `gui_get_key` / `gui_get_mouse`, and display geometry is obtained from the ADE.
