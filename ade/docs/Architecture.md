# ADE Architecture

## Overview

ADE (Advanced Desktop Environment) is a monolithic desktop shell for SARGA OS. It runs as a single process (`no_std` + `alloc`) layered over the libsarga GUI toolkit. ADE owns every active subsystem — window manager, compositor, services, IPC, accessibility — through the single `Desktop` coordinator struct.

## Module Dependency Tree

```
ade (bin)
├── main.rs              — entrypoint, event loop, render dispatch
├── desktop.rs           — Desktop coordinator (~1675 lines)
│   ├── Desktop          — owns all subsystems, event dispatch, layout
│   │
│   ├── window_manager   — ordered window list, focus, drag, resize
│   ├── window           — AppWindow struct, draw(), VisualFlags
│   ├── desktop_icons    — desktop icon grid, select, rubber-band
│   ├── start_menu       — categorized app launcher with search
│   ├── taskbar          — window buttons + clock + tray
│   ├── tray             — system tray entries
│   │
│   ├── service/
│   │   ├── service_manager — owns Notification, Clipboard, Session, Power
│   │   ├── notification    — notification queue (max 64), urgency, timeout
│   │   ├── clipboard       — text buffer + 16-entry history
│   │   ├── session         — boot/login tracking, shutdown/restart/logout
│   │   └── power           — battery status, idle tracking, suspend
│   │
│   ├── ipc/
│   │   ├── server        — channels, pending messages
│   │   ├── registry      — ServiceRegistry + ServiceInfo
│   │   ├── message       — Message, MessageBus, IpcMessage types
│   │   ├── channel       — Channel with subscribers
│   │   ├── client        — IpcClient
│   │   ├── request       — ServiceRequest
│   │   ├── response      — ServiceResponse
│   │   └── permission    — permission constants + PermissionSet
│   │
│   ├── portal/           — request dispatch from IPC to DesktopAPI
│   ├── desktop_api/      — 8 API modules with permission checks
│   │   ├── clipboard, notification, window, launcher
│   │   ├── theme, settings, session, power
│   │
│   ├── a11y/             — A11yTree, FocusManager, node roles
│   ├── render/
│   │   ├── compositor    — 6 layer buffers, Canvas, compose()
│   │   ├── layer         — Layer enum, LAYER_COUNT
│   │   ├── snapshot      — RenderSnapshot (frame capture)
│   │   ├── clock         — ClockCache, format_time
│   │   ├── overlay       — context menu, clipboard panel, switcher
│   │   └── notification_overlay — notification rendering
│   │
│   ├── apps/             — in-process app states
│   │   ├── terminal, files, settings, task_manager, about, tooltip, config_store
│   │
│   ├── theme_service     — ThemeService (wraps libsarga::theme::Theme)
│   ├── launcher          — process fork + window setup
│   ├── perms             — PermissionManager (pid → PermissionSet)
│   ├── shortcut          — ShortcutManager
│   ├── settings          — SettingsState (sound, theme toggle)
│   ├── config            — ConfigManager
│   ├── geometry          — Point, Rect
│   ├── damage            — DamageTracker (full/dirty regions)
│   ├── constants         — TASKBAR_H
│   ├── wallpaper         — wallpaper drawing
│   ├── explorer          — file explorer state
│   ├── icons             — icon bitmaps
│   ├── app_db            — built-in app definitions
│   ├── app_registry      — AppRegistry + AppId + AppEntry
│   ├── app_manifest      — AppManifest
│   ├── app_lifecycle     — AppLifecycleManager
│   ├── desktop_entry     — DesktopEntry parser
│   └── desktop_api       — API module declarations
│
├── constants.rs          — shared constants
├── event.rs              — Event enum (33 variants)
├── geometry.rs           — Point, Rect
├── damage.rs             — DamageTracker
├── config.rs             — ConfigManager
├── wallpaper.rs          — wallpaper rendering
├── icons.rs              — icon lookup
├── app_db.rs             — built-in app catalog
├── app_registry.rs       — AppRegistry
├── shortcut.rs           — ShortcutManager
├── settings.rs           — quick settings toggle
├── theme_service.rs      — ThemeService
├── launcher.rs           — process spawn
├── vfs.rs                — VfsContext
├── file_assoc.rs         — FileAssociationEngine
├── watcher.rs            — FileWatcher
├── explorer.rs           — ExplorerState
├── crash_manager.rs      — CrashManager
├── recovery.rs           — RecoverySystem
├── desktop_entry.rs       — .desktop entry parser
├── desktop_api/mod.rs    — API module declarations
│
├── [scaffolded]          — future phase (dead_code allowed)
│   ├── audio.rs, automation.rs, developer.rs, display.rs
│   ├── extension.rs, input.rs, network.rs, plugin.rs, sdk.rs
│   ├── clipboard_service.rs, login_session.rs, notification.rs
│   ├── power.rs, service_manager.rs, session_service.rs
```

## Data Flow

### Event → Desktop → WindowManager → App → Render → Compositor → Framebuffer

```
Input Event
    ↓
Desktop.handle_event(Event)         — match on variant
    ├─ MouseClick → Desktop.handle_click()
    │   ├─ taskbar click → start_menu / window focus
    │   ├─ icon click → selection / drag
    │   ├─ window click → bring to front, begin drag
    │   └─ titlebar button → close/maximize/minimize
    ├─ Key → shortcuts → subsystem dispatch
    ├─ Drag → move/resize/rubber band
    └─ Scroll → window scroll
    ↓
Desktop.snapshot()                   — capture render state
    ↓
RenderSnapshot                       — read-only borrows
    ↓
render::render()                     — per-layer drawing
    ├─ Layer::Wallpaper  → wallpaper::draw()
    ├─ Layer::Desktop    → desktop_icons::draw()
    ├─ Layer::Windows    → window::draw() × N
    ├─ Layer::Popups     → taskbar::draw() + start_menu::draw()
    ├─ Layer::Overlay    → overlay, notifications, panels
    └─ Layer::Cursor     → (implicitly handled)
    ↓
Compositor::compose()                — alpha-blend layers
    ↓
libsarga::gui::Window::flush()       — framebuffer → display
```

### IPC Flow: App → Portal → DesktopAPI → Service

```
External Process
    ↓  (syscall or pipe)
IpcServer.receive()
    ↓
Portal::dispatch(desktop, app_id, request)
    ↓  (match on service type)
clipboard::handle_request() / notification::handle_request() / ...
    ↓
DesktopAPI::clipboard::copy() / desktop_api::notification::notify() / ...
    ↓  (permission check via PermissionManager)
ServiceManager::clipboard::copy() / ServiceManager::notifications::notify() / ...
```

## Service Architecture

Four services owned by `ServiceManager`:

| Service | File | Function |
|---------|------|----------|
| NotificationManager | `service/notification.rs` | Queue (max 64), urgency, timeout, dismiss |
| ClipboardManager | `service/clipboard.rs` | Text buffer + 16-entry history |
| SessionManager | `service/session.rs` | Boot/login tracking, shutdown/restart/logout |
| PowerManager | `service/power.rs` | Battery status, idle tracking, suspend |

Services tick once per frame via `ServiceManager::tick(current_tick)`.

## Memory Model

- `#![no_std]` + `extern crate alloc;` — no kernel allocator dependency at source level
- Heap-allocated: `Vec<T>` for dynamic collections (windows, notifications, channels)
- Fixed-size: compositor layer buffers (pre-allocated at full resolution), profiler, logger
- No `dyn` trait objects — static dispatch via enums and generics
- No `unwrap()` / `expect()` in production paths — `Result` returns preferred
- Pattern: `core::mem::take()` for zero-copy message drain

## Key Design Decisions

1. **Single Desktop coordinator** — all subsystems owned by Desktop. No DI or plugin architecture in alpha.
2. **Vec-based window storage** — windows stored in insertion order. `bring_to_front` removes + pushes to end.
3. **No event queue** — events dispatched directly in the event loop. Simpler but means no event replay.
4. **Portal pattern** — IPC requests route through a portal layer between IPC server and DesktopAPI for permission enforcement.
5. **Layer compositor** — six screen-sized pixel buffers blend in fixed order. Simple but memory-intensive.
6. **In-process apps** — Terminal, Files, Settings, Task Manager all run as state within ADE (not external processes).
7. **Manual dirty tracking** — `DamageTracker` marks full or partial regions. No automatic invalidation.
8. **No GPU** — software compositor using CPU pixel ops. Floating-point used in gradient rendering (acceptable for alpha).
