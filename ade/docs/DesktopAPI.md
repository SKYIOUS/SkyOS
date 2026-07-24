# Desktop API Reference

## Overview

The Desktop API is the approved interface for interacting with the ADE desktop environment from external applications. It consists of 8 modules in `desktop_api/`, each with permission-gated functions.

## Modules

### Clipboard API (`desktop_api/clipboard.rs`)

Permission: `PERM_CLIPBOARD` (0x0001)

| Function | Signature | Description |
|----------|-----------|-------------|
| `copy` | `(desktop, app, text: &[u8])` | Copy text to clipboard |
| `paste` | `(desktop, app) -> Option<&str>` | Paste text from clipboard |

### Notification API (`desktop_api/notification.rs`)

Permission: `PERM_NOTIFICATIONS` (0x0002)

| Function | Signature | Description |
|----------|-----------|-------------|
| `notify` | `(desktop, app, title, body, urgency: u8, timeout: u32)` | Show notification |
| `dismiss` | `(desktop, app, id: u64)` | Dismiss notification by ID |
| `dismiss_all` | `(desktop, app)` | Dismiss all notifications |

### Window API (`desktop_api/window.rs`)

Permission: `PERM_WINDOW_CONTROL` (0x0008)

| Function | Signature | Description |
|----------|-----------|-------------|
| `create` | `(desktop, app, title, x, y, w, h) -> Option<WindowId>` | Create application window |
| `close` | `(desktop, app, wid: WindowId)` | Close a window |
| `focus` | `(desktop, app, wid: WindowId)` | Bring window to front |

### Launcher API (`desktop_api/launcher.rs`)

No permission check (requires `PERM_EXEC` implicitly).

| Function | Signature | Description |
|----------|-----------|-------------|
| `launch` | `(desktop, app, path, title)` | Launch application |
| `launch_at` | `(desktop, app, path, title, x, y, w, h)` | Launch at position |

### Theme API (`desktop_api/theme.rs`)

Permission: `PERM_SETTINGS` (0x0010)

| Function | Signature | Description |
|----------|-----------|-------------|
| `current` | `(desktop, app) -> &Theme` | Get current theme |
| `set_dark` | `(desktop, app)` | Switch to dark theme |
| `set_light` | `(desktop, app)` | Switch to light theme |

### Settings API (`desktop_api/settings.rs`)

Permission: `PERM_SETTINGS` (0x0010)

| Function | Signature | Description |
|----------|-----------|-------------|
| `open` | `(desktop, app)` | Open settings panel |
| `close` | `(desktop, app)` | Close settings panel |
| `is_open` | `(desktop, app) -> bool` | Check if settings is open |

### Session API (`desktop_api/session.rs`)

Permission: `PERM_POWER` (0x0020)

| Function | Signature | Description |
|----------|-----------|-------------|
| `uptime` | `(desktop, app) -> u64` | Get system uptime in ticks |
| `shutdown` | `(desktop, app)` | Request system shutdown |
| `restart` | `(desktop, app)` | Request system restart |
| `logout` | `(desktop, app)` | Request user logout |

### Power API (`desktop_api/power.rs`)

Permission: `PERM_POWER` (0x0020) for state-changing functions. Read-only functions are unconditional.

| Function | Signature | Description |
|----------|-----------|-------------|
| `battery_available` | `(desktop, app) -> bool` | Battery present |
| `battery_percentage` | `(desktop, app) -> u8` | Battery level (0–100) |
| `ac_connected` | `(desktop, app) -> bool` | AC power connected |
| `request_suspend` | `(desktop, app)` | Request system suspend |

## Permission Requirements Summary

| API Module | Permission Required |
|------------|-------------------|
| Clipboard | PERM_CLIPBOARD (0x0001) |
| Notification | PERM_NOTIFICATIONS (0x0002) |
| Window | PERM_WINDOW_CONTROL (0x0008) |
| Launcher | None (implicit EXEC) |
| Theme | PERM_SETTINGS (0x0010) |
| Settings | PERM_SETTINGS (0x0010) |
| Session | PERM_POWER (0x0020) for state changes |
| Power | PERM_POWER (0x0020) for state changes |

## Usage Examples

```rust
// External application requests clipboard copy
let msg = Message {
    id: MessageId(1),
    sender: ApplicationId(pid),
    receiver: ApplicationId(0),
    msg_type: MessageType::Request,
    payload: MessagePayload::Data(b"Hello World".to_vec()),
    timestamp: 0,
    flags: 0,
};
ipc_server.send(msg);
```
