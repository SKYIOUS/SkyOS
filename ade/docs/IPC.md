# IPC Subsystem

## Architecture

The IPC subsystem provides message-based communication between ADE and external applications (or internal components). It uses a server-centric model with channels, message queues, and a service registry.

## Components

### Message Types (`ipc/message.rs`)

```rust
pub(crate) struct Message {
    pub id: MessageId,         // unique message ID
    pub sender: ApplicationId, // source application
    pub receiver: ApplicationId, // target application
    pub msg_type: MessageType, // Request/Reply/Broadcast/Notification/Signal
    pub payload: MessagePayload, // None/Data/Text
    pub timestamp: u64,
    pub flags: u32,
}
```

Legacy types (backward compatible):
- `IpcMessage` — enum of Request/Response/Broadcast
- `IpcRequest` — seq, target, method, args
- `IpcResponse` — seq, success, data
- `IpcBroadcast` — topic, data
- `IpcTarget` — Window/Application/Service/Desktop

### Message Bus

`MessageBus` in `message.rs`:
- Owns a list of pending `IpcMessage`s
- `request()` — creates a Request message and returns sequence number
- `respond()` — creates a Response
- `broadcast()` — creates a Broadcast
- `drain()` — zero-copy take of pending queue (`core::mem::take`)

### Channels (`ipc/channel.rs`)

```rust
pub(crate) struct Channel {
    pub id: ChannelId,
    pub channel_type: ChannelType, // RequestReply/Broadcast/Notification/Signal/OneToMany/ManyToOne
    pub subscribers: Vec<ApplicationId>,
    pub messages: Vec<Message>,
}
```

- `subscribe(app)` — add application to subscriber list
- `unsubscribe(app)` — remove from subscribers
- `push(msg)` — enqueue message
- `drain()` — zero-copy drain

### Server (`ipc/server.rs`)

```rust
pub(crate) struct IpcServer {
    pub channels: Vec<Channel>,
    pub next_channel_id: u64,
    pub next_message_id: u64,
    pub pending_messages: Vec<Message>,
}
```

- `create_channel(channel_type)` → ChannelId
- `subscribe(channel_id, app)` → bool
- `unsubscribe(channel_id, app)` → bool
- `send(msg)` — push to pending
- `drain_pending()` — zero-copy take
- `tick()` — no-op (reserved for future maintenance)

### Service Registry (`ipc/registry.rs`)

```rust
pub(crate) enum ServiceId {
    Clipboard, Notification, Launcher, FileDialog,
    Settings, Session, Window, Theme, Power,
}

pub(crate) struct ServiceInfo {
    pub id: ServiceId,
    pub name: &'static str,
    pub version: u32,
    pub required_permissions: u32,
    pub available: bool,
}
```

- `find(id)` — lookup by ServiceId
- `find_by_name(name)` — lookup by string name
- `register(info)` — register service
- `set_available(id, available)` — mark service availability
- `discover(name)` / `discover_by_permission(perm)` — discovery API

## Permission Model

Defined in `ipc/permission.rs`:

| Constant | Bit | Description |
|----------|-----|-------------|
| PERM_CLIPBOARD | 0x0001 | Read/write clipboard |
| PERM_NOTIFICATIONS | 0x0002 | Send/receive notifications |
| PERM_FILESYSTEM | 0x0004 | File system access |
| PERM_WINDOW_CONTROL | 0x0008 | Create/manage windows |
| PERM_SETTINGS | 0x0010 | Modify system settings |
| PERM_POWER | 0x0020 | Power management |
| PERM_CAMERA | 0x0040 | Camera access |
| PERM_MICROPHONE | 0x0080 | Microphone access |
| PERM_NETWORK | 0x0100 | Network access |
| PERM_USB | 0x0200 | USB device access |
| PERM_BLUETOOTH | 0x0400 | Bluetooth |
| PERM_LOCATION | 0x0800 | Location services |

`PermissionSet` — bitmask wrapper with `grant()`, `revoke()`, `check()`, `has_any()`.

## Portal Dispatch

`portal/mod.rs` routes IPC requests to DesktopAPI:

```
IPC Request → Portal::dispatch() → match service:
  Clipboard   → portal::clipboard::handle_request()   → desktop_api::clipboard::*()
  Notification → portal::notification::handle_request() → desktop_api::notification::*()
  Settings    → portal::settings::handle_request()      → desktop_api::settings::*()
  Window      → portal::window::handle_request()        → desktop_api::window::*()
  FileDialog  → portal::file_dialog::handle_request()   → (future)
  _           → error response
```

## Message Flow Examples

### Clipboard Copy
```
App → [Message: Copy(text)] → IpcServer → Portal
  → clipboard::handle_request()
  → PermissionManager.check(pid, PERM_CLIPBOARD)
  → ClipboardManager.copy(text)
  → [Response: ok]
```

### Notification Send
```
App → [Message: Notify(title, body, urgency, timeout)] → IpcServer → Portal
  → notification::handle_request()
  → PermissionManager.check(pid, PERM_NOTIFICATIONS)
  → NotificationManager.notify(title, body, urgency, timeout)
  → [Response: {notification_id}]
```

### Window Create
```
App → [Message: CreateWindow(title, x, y, w, h)] → IpcServer → Portal
  → window::handle_request()
  → PermissionManager.check(pid, PERM_WINDOW_CONTROL)
  → desktop_api::window::create()
  → WindowManager.create()
  → [Response: {window_id}]
```
