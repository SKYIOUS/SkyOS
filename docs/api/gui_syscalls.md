# GUI Subsystem Syscalls

SkyOS provides kernel-level syscalls for the in-kernel compositor (`src/gui/`). Each window is
identified by a **handle** (an index into the compositor's window list). Userspace typically calls
these through `libsarga::gui::Window` (`libsarga/src/gui.rs`).

Syscall numbers follow Linux x86_64 convention where applicable (see `SYSCALL_ABI.md` and
`kernel/src/syscalls/numbers.rs`).

## gui_create_window (100)

```c
u64 gui_create_window(const char* title, usize width, usize height);
```

Creates a window with the given title and content size. The compositor allocates a shared physical
framebuffer (`width * height` pixels, 32-bit) or falls back to an in-memory content buffer. Returns
the window handle, or a negative errno on failure.

## gui_get_buffer (101)

```c
u64 gui_get_buffer(u64 handle);
```

Returns the content area size of the window, packed as `width` in the low 32 bits and `height` in the
high 32 bits. Returns 0 if the handle is invalid. (The user buffer itself comes from
`gui_map_buffer`.)

## gui_map_buffer (103)

```c
u64 gui_map_buffer(u64 handle);
```

Maps the window's shared framebuffer into the calling process's address space and returns the virtual
address (0 on failure). The region is registered as a VMA; pages are user-accessible and writable.
`libsarga::gui::Window::create` calls this and wraps the result as `&mut [u32]`.

## gui_flush (102)

```c
u64 gui_flush(u64 handle, const u32* buf);
```

Copies `buf` into the window's content buffer (no-op for physically-backed windows, which are drawn
directly) and triggers a compositor render with the current cursor position. Returns 0 on success, or
`EBADF`/`ENOSYS` for invalid handles.

## gui_get_key (105)

```c
u64 gui_get_key(u64 handle);
```

Pops the next queued key event for the window (a keycode) or 0 if the queue is empty.

## gui_get_mouse (120)

```c
u64 gui_get_mouse(u64 handle);
```

Returns the mouse state relative to the window's content area, packed as `x` (low 16 bits), `y`
(bits 16-31), `buttons` (bits 32-39), `scroll` (bits 40-47). Returns 0 if the handle is invalid.

## gui_set_title (121)

```c
u64 gui_set_title(u64 handle, const char* title);
```

Updates the window title (capped at 64 bytes). Returns 0 on success or `EINVAL` for an invalid handle
or null pointer.

## gui_destroy_window (122)

```c
u64 gui_destroy_window(u64 handle);
```

Removes the window from the compositor. Returns 0 on success or `EINVAL` for an invalid handle.

## gui_resize_window (123)

```c
u64 gui_resize_window(u64 handle, u64 width, u64 height);
```

Resizes the window. Returns 0 on success or `EINVAL` for an invalid handle.

## gui_move_window (124)

```c
u64 gui_move_window(u64 handle, u64 x, u64 y);
```

Moves the window. Returns 0 on success or `EINVAL` for an invalid handle.

## beep (104)

```c
u64 beep(u32 freq_hz, u32 duration_ms);
```

Emits a PC-speaker tone. See `libsarga` `beep()`.

## clipboard (125)

```c
u64 clipboard(u64 mode, u8* buf, u64 len);
```

Access the compositor clipboard. `mode` 0 = read into `buf` (returns bytes copied), 1 = write `buf`
(returns `len`), 2 = get clipboard length.

## notify (126)

```c
u64 notify(const char* text, u64 duration_ms, u64 kind);
```

Queues a desktop notification. `kind` 0 = Info, 1 = Warning, 2 = Error (text capped at 256 bytes,
minimum 100 ms display time). Returns 0 on success or `EINVAL` for a null/non-UTF-8 text.

## Event Handling

Input events are not delivered through a device file; applications poll `gui_get_key` /
`gui_get_mouse` per frame from their event loop (the ADE desktop polls at ~60 Hz).
