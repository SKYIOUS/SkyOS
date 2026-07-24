# Services Reference

## Overview

ADE provides four built-in services, all owned by `ServiceManager` (`service/service_manager.rs`). They are ticked once per frame and provide core desktop functionality.

## Service Manager

```rust
pub(crate) struct ServiceManager {
    pub notifications: NotificationManager,
    pub clipboard: ClipboardManager,
    pub session: SessionManager,
    pub power: PowerManager,
}
```

- `new(boot_tick)` — initialize all services
- `tick(current_tick)` — tick notifications and power manager
- `notify(title, body, urgency, timeout)` — convenience method for notification creation

## NotificationManager

File: `service/notification.rs`

### Structure
```rust
pub(crate) struct NotificationManager {
    notifications: Vec<Notification>,
    next_id: u64,
    visible_count: usize,
}
```

### Limits
- Max 64 notifications in queue
- Max 16 concurrent visible notifications

### API
| Method | Description |
|--------|-------------|
| `notify(title, body, urgency, timeout)` → `u64` | Create notification, return ID |
| `notify_with_icon(title, body, icon_id, urgency, timeout)` → `u64` | Create with icon |
| `dismiss(id)` → `bool` | Mark notification dismissed |
| `dismiss_all()` | Dismiss all |
| `update(id, title, body)` | Update existing notification |
| `tick(current_tick)` | Auto-dismiss timed-out notifications |
| `visible_notifications()` → `&[Notification]` | Get non-dismissed notifications |

### Dismissal Strategy
Dismissed notifications are swapped to the end of the visible range to keep the active list contiguous.

### Timeout
Seconds = `timeout / 30` (since clock ticks at ~30Hz).

## ClipboardManager

File: `service/clipboard.rs`

### Structure
```rust
pub(crate) struct ClipboardManager {
    pub text: String,         // current clipboard content
    pub length: usize,
    pub timestamp: u64,
    history: Vec<ClipboardEntry>,  // 16-entry history
}
```

### API
| Method | Description |
|--------|-------------|
| `copy(text, timestamp)` | Store text + push to history |
| `paste()` → `&str` | Get current clipboard |
| `clear()` | Clear clipboard |
| `history()` → `&[ClipboardEntry]` | Get history |
| `is_empty()` → `bool` | Check if empty |

### History Behavior
- Max 16 entries
- Duplicates removed (retain then push)
- Oldest entry removed when over capacity

## SessionManager

File: `service/session.rs`

### Structure
```rust
pub(crate) struct SessionManager {
    boot_tick: u64,
    login_tick: u64,
    pub shutdown_requested: bool,
    pub restart_requested: bool,
    pub logout_requested: bool,
    pub desktop_state_saved: bool,
    pub recent_apps: VecDeque<u64>,
}
```

### API
| Method | Description |
|--------|-------------|
| `uptime(current_tick)` → `u64` | Ticks since boot |
| `session_duration(current_tick)` → `u64` | Ticks since login |
| `request_shutdown()` | Flag shutdown |
| `request_restart()` | Flag restart |
| `request_logout()` | Flag logout |
| `mark_state_saved()` | Mark desktop state persisted |
| `record_app_launch(app_id)` | Track recent apps (max 10) |
| `reset(current_tick)` | Reset session state |

## PowerManager

File: `service/power.rs`

### Structure
```rust
pub(crate) struct PowerManager {
    pub battery_available: bool,
    pub battery_percentage: u8,
    pub ac_connected: bool,
    pub suspend_requested: bool,
    pub shutdown_requested: bool,
    pub restart_requested: bool,
    pub sleep_requested: bool,
    idle_ticks: u64,
    last_activity_tick: u64,
}
```

### API
| Method | Description |
|--------|-------------|
| `mark_activity(tick)` | Reset idle counter |
| `tick(current_tick)` | Update idle duration |
| `request_suspend()` | Flag suspend |
| `request_shutdown()` | Flag shutdown |
| `request_restart()` | Flag restart |
| `request_sleep()` | Flag sleep |
| `set_battery(available, percentage, ac)` | Update battery status |

## Service Lifecycle

Services are initialized in `ServiceManager::new()` with default state. They receive ticks every frame. The `Desktop` coordinator calls `services.tick()` during its `tick()` method, which in turn calls:

```
ServiceManager::tick(current_tick)
  ├── NotificationManager::tick(current_tick) — timeout auto-dismiss
  └── PowerManager::tick(current_tick) — idle tracking
```

## Event Dispatch

Services are also indirectly notified of changes through the `Event` system. Events like `Event::ClipboardChanged`, `Event::NotificationAdded`, and `Event::PowerRequest` are defined but currently handled as no-ops in `Desktop::handle_event()`.
