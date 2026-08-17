# ADE Desktop Environment Architecture: Core, IPC, Security, and System Services

**Codemap ID:** ADE_Desktop_Environment_Architecture__Core__IPC__Security__and_System_Services_20260731_113253

**Description:** Maps the restructured ADE desktop environment showing event-driven coordination, IPC service registry with security portals, window management lifecycle, and compositor-based rendering. Key entry points: application launch [1c], IPC service dispatch [3b], permission validation [4b], and frame rendering [5c].

---

## Trace 1: Application Launch: Start Menu to Process Creation

**Description:** Core desktop service - traces user click through app registry lookup, process fork/exec, window creation, and lifecycle registration

### Flow Diagram

```
Application Launch Flow (Trace 1)
├── Main Event Loop (main.rs)
│   ├── get_mouse() input polling <-- main.rs:55
│   └── handle_event(MouseClick) <-- 1a
│       └── Desktop Event Handler (desktop.rs)
│           ├── handle_click() UI hit testing <-- desktop.rs:1137
│           │   └── Start menu click detected
│           │       └── launch_app(app_id) <-- 1b
│           │           ├── app_reg.get() registry lookup <-- desktop.rs:817
│           │           └── spawn_app_from_registry() <-- 1c
│           │               └── Launcher (launcher.rs)
│           │                   ├── process::fork() <-- 1d
│           │                   │   ├── Child: execve() <-- 1e
│           │                   │   └── Parent continues
│           │                   ├── wm.create(app_win) <-- 1f
│           │                   ├── lifecycle.register() <-- 1g
│           │                   └── services.notify() <-- 1h
└── Result: Process running + window visible
```

### Locations

| ID | Title | Description | Path:LineNumber |
|----|-------|-------------|-----------------|
| 1a | Event Loop Entry | Main event loop dispatches mouse click to desktop coordinator | `ade/src/main.rs:62` |
| 1b | App Launch Trigger | Start menu click handler initiates app launch from registry | `ade/src/core/desktop.rs:1212` |
| 1c | Registry-Based Spawn | Desktop delegates to launcher with app metadata from registry | `ade/src/core/desktop.rs:847` |
| 1d | Process Fork | Launcher forks new process for application isolation | `ade/src/core/launcher.rs:94` |
| 1e | Binary Execution | Child process replaces itself with application binary | `ade/src/core/launcher.rs:96` |
| 1f | Window Creation | Window manager creates and tracks new application window | `ade/src/core/launcher.rs:116` |
| 1g | Lifecycle Registration | System lifecycle manager tracks process for crash detection and restart policy | `ade/src/core/launcher.rs:106` |
| 1h | Launch Notification | Service manager sends notification confirming app launch | `ade/src/core/launcher.rs:121` |

### Code Snippets

**File: ade/src/main.rs (Lines 60-64)**
```rust
        }
        if pressed {
            desktop.handle_event(core::event::Event::MouseClick(ms.x as i32, ...
        } else if ms.buttons & 4 != 0 {
            desktop.handle_event(core::event::Event::MouseMiddle(ms.x as i32,...
```

**File: ade/src/core/desktop.rs (Lines 1210-1214)**
```rust
                if mx >= list_x && mx < list_x + list_w && my >= iy && my < i...
                    let app_id = self.start_menu.filtered[i];
                    self.launch_app(app_id);
                    return;
                }
```

**File: ade/src/core/desktop.rs (Lines 845-849)**
```rust
            return;
        }
        crate::core::launcher::spawn_app_from_registry(self, &app);
        self.damage.mark_full();
```

**File: ade/src/core/launcher.rs (Lines 94-115)**
```rust
    if !path.is_empty() {
        let ipc_pair = libsarga::net::socketpair(
            libsarga::net::SocketDomain::Unix as u64,
            libsarga::net::SocketType::Stream as u64,
            0,
        )
        .ok();
        match libsarga::process::fork() {
            Ok(0) => {
                match ipc_pair {
                    Some((server_fd, client_fd)) => {
                        let _ = libsarga::io::close(server_fd);
                        let fd_arg = alloc::format!("{}", client_fd);
                        let argv = [path, "--ipc-fd", fd_arg.as_str()];
                        let _ = libsarga::process::execve(path, &argv, &[]);
                    }
                    None => {
                        let _ = libsarga::process::execve(path, &[path], &[]);
                    }
                }
                libsarga::process::exit(1);
            }
```

**File: ade/src/core/launcher.rs (Lines 104-108)**
```rust
                    .map(|id| id.0)
                    .unwrap_or(0);
                desktop.lifecycle.register(pid, app_idx);
                app_win
                    .content
```

**File: ade/src/core/launcher.rs (Lines 114-123)**
```rust
        }
    }
    let id = desktop.wm.create(app_win);
    if let Some(w) = desktop.wm.lookup_mut(id) {
        w.flags.opacity = 0;
        w.animate_to(w.x, w.y, w.w, w.h);
    }
    desktop.services.notify("App Launched", title, 1, 120);
    desktop.damage.mark_full();
```

---

## Trace 2: Window Manager: Create, Focus, and Render Cycle

**Description:** Core window management - shows window creation, focus management, state transitions, and integration with rendering pipeline

### Flow Diagram

```
Window Manager Lifecycle
├── WindowManager::create() <-- window_manager.rs:45
│   ├── windows.push(window) <-- 2a
│   └── focused = Some(id.0) <-- 2b
├── WindowManager::bring_to_front() <-- window_manager.rs:101
│   ├── windows.remove(id.0) <-- 2c
│   └── windows.push(w) <-- 2d
├── AppWindow animation system
│   ├── animate_to() creates AnimState <-- 2e
│   └── tick_animation() interpolates <-- 2f
└── Rendering pipeline
    └── render::render() <-- mod.rs:14
        └── window::draw(&mut cv, ...) <-- 2g
```

### Locations

| ID | Title | Description | Path:LineNumber |
|----|-------|-------------|-----------------|
| 2a | Window Registration | WindowManager adds new window to managed collection | `ade/src/core/window_manager.rs:46` |
| 2b | Focus Assignment | Newly created window receives immediate focus | `ade/src/core/window_manager.rs:48` |
| 2c | Z-Order Manipulation | Bring-to-front removes window from current position for re-insertion | `ade/src/core/window_manager.rs:105` |
| 2d | Top Stack Placement | Window pushed to end of vector for top z-order rendering | `ade/src/core/window_manager.rs:110` |
| 2e | Animation State | Window initiates smooth transition animation for position/size changes | `ade/src/core/window.rs:97` |
| 2f | Animation Tick | Per-frame animation update interpolates window geometry | `ade/src/core/window.rs:112` |
| 2g | Window Rendering | Compositor canvas receives window draw commands with theme and state | `ade/src/core/window.rs:164` |

### Code Snippets

**File: ade/src/core/window_manager.rs (Lines 50-64)**
```rust
    /// WindowManager API v1.0
    pub fn create(&mut self, mut window: AppWindow) -> WindowId {
        window.id = self.next_id;
        let id = WindowId(window.id);
        self.next_id += 1;
        self.windows.push(window);
        self.focused = Some(id.0);
        id
    }
```

WindowId is a stable u64 (monotonic `next_id` counter), not a Vec index. All lookups resolve ids to positions via `find_index()` linear scan — windows count is small. `focused`/`dragging` store stable ids so they survive window removal.

**File: ade/src/core/window_manager.rs (Lines 103-112)**
```rust
            return;
        }
        let mut w = self.windows.remove(id.0);
        w.focused = true;
        for other in &mut self.windows {
            other.focused = false;
        }
        self.windows.push(w);
        self.focused = Some(id.0);
    }
```

**File: ade/src/core/window.rs (Lines 95-99)**
```rust
impl AppWindow {
    pub(crate) fn animate_to(&mut self, x: i32, y: i32, w: u32, h: u32) {
        self.anim = Some(AnimState {
            from_x: self.x,
            from_y: self.y,
```

**File: ade/src/core/window.rs (Lines 110-114)**
```rust

    pub(crate) fn tick_animation(&mut self) -> bool {
        if let Some(ref mut a) = self.anim {
            a.tick += 1;
            let t = a.tick.min(a.duration);
```

**File: ade/src/core/window.rs (Lines 162-166)**
```rust
    cursor_visible: bool,
    explorers: &[crate::util::explorer::ExplorerState],
) {
    // Don't draw minimized windows (but still draw during animation).
    if aw.state == WindowState::Minimized && aw.anim.is_none() {
```

---

## Trace 3: IPC Service Request: Portal Dispatch and Security

**Description:** IPC subsystem - demonstrates service registry lookup, portal-based request routing, permission validation, and response handling

### Flow Diagram

```
IPC Service Request Flow
├── IPC Server Layer
│   ├── IpcServer::submit_request(req) <-- server.rs
│   │   └── pending_requests.push(req) <-- 3a
│   └── IpcServer::send(Message) <-- server.rs:56 (channel path)
│       └── pending_messages.push(msg)
├── Desktop::process_ipc() <-- desktop.rs (per frame, from tick)
│   └── permission gate: granted ∩ required_permissions
│       └── denied → ServiceResponse{ success:false }
├── Security Portal Dispatcher
│   └── portal::dispatch() <-- 3b
│       ├── Match ServiceId <-- 3c
│       │   ├── Clipboard handler
│       │   │   └── clipboard::handle_request() <-- clipboard.rs:6
│       │   │       ├── desktop_api::copy() <-- 3d
│       │   │       └── ServiceResponse { ... } <-- 3e
│       │   ├── Notification handler <-- mod.rs:13
│       │   ├── Window handler <-- mod.rs:15
│       │   └── Other service handlers
│       └── Return ServiceResponse
└── Service Registry
    ├── ServiceRegistry::register_defaults() <-- registry.rs (from Desktop::new)
    └── ServiceRegistry::find() <-- registry.rs:41
        └── services.iter().find() <-- 3f
```

> Wiring: service requests are submitted via `IpcServer::submit_request` and drained
> once per frame by `Desktop::process_ipc`, which gates each request on the
> service's `required_permissions` against the sender's grant, then routes through
> the security portal. Responses collect on `IpcServer::pending_responses`.
>
> Transport: each externally spawned app gets an AF_UNIX socketpair (`libsarga::net::socketpair(1, 1, 0)`). The server end lives in `IpcTransport` (ade/src/ipc/transport.rs); the client end is inherited across fork+exec and passed to the child as `--ipc-fd <n>`. Each frame is `u32 LE length | payload`, `<= 4096` bytes; one write = one queued datagram = one read pops it whole. Requests never carry a sender — the server maps fd → pid (authoritative identity for the permission gate). The kernel reports `POLLIN` on unix sockets only when data is queued, so the server can poll(timeout 0) then read without blocking.

### Locations

| ID | Title | Description | Path:LineNumber |
|----|-------|-------------|-----------------|
| 3a | Message Queuing | IPC server queues incoming service request message | `ade/src/ipc/server.rs:57` |
| 3b | Portal Dispatch Entry | Security portal layer routes requests to appropriate service handlers | `ade/src/sec/portal/mod.rs:10` |
| 3c | Service Routing | Portal matches service ID to specific handler implementation | `ade/src/sec/portal/mod.rs:12` |
| 3d | Clipboard Operation | Portal handler delegates to desktop API with app context | `ade/src/sec/portal/clipboard.rs:9` |
| 3e | Response Construction | Portal builds IPC response with request correlation ID | `ade/src/sec/portal/clipboard.rs:10` |
| 3f | Service Discovery | Registry provides service metadata lookup for capability queries | `ade/src/ipc/registry.rs:42` |

### Code Snippets

**File: ade/src/ipc/server.rs (Lines 55-59)**
```rust
    /// IPC API v1.0
    pub fn send(&mut self, msg: Message) {
        self.pending_messages.push(msg);
    }
```

**File: ade/src/sec/portal/mod.rs (Lines 8-14)**
```rust
use crate::ipc::{ApplicationId, ServiceRequest, ServiceResponse};

pub(crate) fn dispatch(desktop: &mut Desktop, app: ApplicationId, req: &Servi...
    match req.service {
        crate::ipc::ServiceId::Clipboard => clipboard::handle_request(desktop...
        crate::ipc::ServiceId::Notification => notification::handle_request(d...
        crate::ipc::ServiceId::Settings => settings::handle_request(desktop, ...
```

**File: ade/src/sec/portal/clipboard.rs (Lines 7-12)**
```rust
    match req.method {
        "copy" => {
            crate::util::desktop_api::clipboard::copy(desktop, app, &req.args);
            ServiceResponse { request_id: req.request_id, success: true, data...
        }
        "paste" => {
```

**File: ade/src/ipc/registry.rs (Lines 40-44)**
```rust

    pub fn find(&self, id: ServiceId) -> Option<&ServiceInfo> {
        self.services.iter().find(|s| s.id == id)
    }
```

---

## Trace 4: Permission System: Registration and Validation

**Description:** Security layer - shows permission set management, per-process registration, and runtime capability checks

### Flow Diagram

```
Permission System Architecture
├── PermissionManager (sec/perms.rs) — single permission type: AppPermission
│   ├── register(pid, perms: AppPermission) <-- launcher::spawn_app_at
│   │   └── app_perms.push((pid, perms)) <-- 4a
│   ├── check(pid, perm: AppPermission) <-- desktop.permission_check
│   │   └── find process & validate <-- 4b
│   │       └── set.contains(perm) <-- 4d
│   ├── granted(pid) -> Option<AppPermission> <-- desktop.process_ipc
│   └── unregister(pid) <-- desktop.reap_children
├── Grant
│   └── default_grant() <-- launcher.rs (flat default; manifests drive per-app grants later)
│       └── on the transport path, identity is fd→pid derived at spawn, not client-declared
└── Service Registry Integration
    └── find_by_permission(perm) <-- registry.rs
        └── filter services by perms <-- 4e
```

### Locations

| ID | Title | Description | Path:LineNumber |
|----|-------|-------------|-----------------|
| 4a | Permission Registration | Launcher grants default AppPermission set when a process starts | `ade/src/core/launcher.rs:107` |
| 4b | Permission Check | `desktop.permission_check` validates capability before gated API calls | `ade/src/sec/perms.rs:24` |
| 4c | Default Grant | Flat default grant (clipboard/notifications/filesystem/window/settings, no power/hardware) | `ade/src/sec/perms.rs:45` |
| 4d | Has Permission | bitflags `contains` validates all required permission bits are set | `ade/src/sec/perms.rs:28` |
| 4e | Service Permission Filter | Registry filters services by required permission capabilities | `ade/src/ipc/registry.rs:50` |
| 4f | Permission Cleanup | Reaped child process's grant is removed from the manager | `ade/src/core/desktop.rs:239` |

### Code Snippets

**File: ade/src/sec/perms.rs (Lines 24-50)**
```rust
/// Permission API v1.0
pub(crate) struct PermissionManager {
    pub app_perms: Vec<(u64, AppPermission)>, // pid → permissions
}

impl PermissionManager {
    pub fn register(&mut self, pid: u64, perms: AppPermission) {
        self.app_perms.push((pid, perms));
    }

    pub fn check(&self, pid: u64, perm: AppPermission) -> bool {
        self.app_perms
            .iter()
            .find(|(p, _)| *p == pid)
            .map(|(_, set)| set.contains(perm))
            .unwrap_or(false)
    }

    pub fn granted(&self, pid: u64) -> Option<AppPermission> {
        self.app_perms
            .iter()
            .find(|(p, _)| *p == pid)
            .map(|(_, set)| *set)
    }

    pub fn unregister(&mut self, pid: u64) {
        self.app_perms.retain(|(p, _)| *p != pid);
    }
}
```

**File: ade/src/core/launcher.rs (Lines 104-109)**
```rust
                desktop.lifecycle.register(pid, app_idx);
                desktop.permissions.register(pid, crate::sec::perms::default_grant());
                desktop.lifecycle.mark_running(pid);
```

**File: ade/src/ipc/registry.rs (Lines 48-52)**
```rust

    pub fn find_by_permission(&self, perm: u32) -> Vec<&ServiceInfo> {
        self.services
            .iter()
            .filter(|s| s.required_permissions & perm == perm)
```

---

## Trace 5: Rendering Pipeline: Snapshot to Compositor

**Description:** Render subsystem - traces frame preparation from desktop state snapshot through layer-based composition to window buffer flush

### Flow Diagram

```
Main Event Loop (main.rs) <-- main.rs:21
├── while running loop <-- main.rs:41
│   ├── desktop.tick() <-- main.rs:42
│   ├── handle input events <-- main.rs:44
│   └── if desktop.damage.is_dirty() <-- 5a
│       ├── desktop.snapshot() <-- 5b
│       ├── render::render() call <-- 5c
│       │   └── Render Pipeline (render/mod.rs) <-- mod.rs:14
│       │       ├── comp.clear_all() <-- 5d
│       │       ├── Layer: Wallpaper
│       │       │   └── wallpaper::draw() <-- mod.rs:25
│       │       ├── Layer: Windows
│       │       │   └── window::draw() <-- 5e
│       │       └── Compositor::compose() <-- mod.rs:208
│       │           ├── copy wallpaper base <-- 5f
│       │           ├── for each layer <-- compositor.rs:792
│       │           │   └── alpha_blend() <-- 5g
│       │           └── output to dst buffer
│       └── desktop_win.flush() <-- 5h
└── sleep_ns() <-- main.rs:94
```

### Locations

| ID | Title | Description | Path:LineNumber |
|----|-------|-------------|-----------------|
| 5a | Damage Check | Main loop conditionally renders only when desktop state changed | `ade/src/main.rs:74` |
| 5b | State Snapshot | Desktop creates immutable snapshot of current frame state | `ade/src/main.rs:76` |
| 5c | Render Invocation | Render pipeline processes snapshot into compositor layers | `ade/src/main.rs:77` |
| 5d | Layer Clear | Compositor resets all layer buffers to transparent | `ade/src/render/mod.rs:20` |
| 5e | Window Layer Draw | Windows rendered to dedicated layer with theme and state | `ade/src/render/mod.rs:53` |
| 5f | Base Layer Copy | Compositor starts with wallpaper as opaque base layer | `ade/src/render/compositor.rs:789` |
| 5g | Alpha Blending | Each layer alpha-blended over accumulated output buffer | `ade/src/render/compositor.rs:802` |
| 5h | Buffer Flush | Composited frame flushed to window system for display | `ade/src/main.rs:78` |

### Code Snippets

**File: ade/src/main.rs (Lines 72-80)**
```rust
        }

        if desktop.damage.is_dirty() {
            let clock_str = desktop.prepare_clock();
            let snap = desktop.snapshot();
            render::render(&mut desktop_win, &snap, &clock_str, &mut composit...
            if let Err(e) = desktop_win.flush() {
                io::print_str(&alloc::format!("[ade] flush error: {}\n", e));
            }
```

**File: ade/src/render/mod.rs (Lines 18-22)**
```rust
    comp: &mut Compositor,
) {
    comp.clear_all();

    // Wallpaper
```

**File: ade/src/render/mod.rs (Lines 51-55)**
```rust
                    cv.draw_shadow(aw.x as u32, aw.y as u32, aw.w, aw.h, 8, 0...
                }
                crate::core::window::draw(&mut cv, snap.theme, aw, snap.curso...
            }
        }
```

**File: ade/src/render/compositor.rs (Lines 787-791)**
```rust
        if full {
            // Start with wallpaper (full-opacity copy).
            dst.copy_from_slice(&self.layers[Layer::Wallpaper as usize].buf);

            // Blend each subsequent layer over the accumulated output.
```

**File: ade/src/render/compositor.rs (Lines 800-804)**
```rust
                        dst[i] = px;
                    } else {
                        dst[i] = alpha_blend(dst[i], px, (px >> 24) as u8);
                    }
                }
```

---

## Trace 6: Service Manager: Initialization and Tick Cycle

**Description:** System services - demonstrates service manager creation, individual service initialization, and per-frame tick propagation

### Flow Diagram

```
Desktop Environment Initialization & Tick Cycle
├── Desktop::new() constructor <-- desktop.rs:145
│   └── ServiceManager::new(boot_tick) <-- 6a
│       ├── NotificationManager::new() <-- 6b
│       ├── ClipboardManager::new() <-- service_manager.rs:19
│       ├── SessionManager::new(boot_tick) <-- 6c
│       └── PowerManager::new() <-- service_manager.rs:21
└── Main Event Loop (main.rs) <-- main.rs:41
    └── desktop.tick() <-- main.rs:42
        └── self.services.tick(clock_ticks) <-- 6d
            ├── notifications.tick(current_tick) <-- 6e
            │   └── timeout check & dismiss <-- 6f
            └── power.tick(current_tick) <-- service_manager.rs:27
```

### Locations

| ID | Title | Description | Path:LineNumber |
|----|-------|-------------|-----------------|
| 6a | Service Manager Init | Desktop constructor creates centralized service manager | `ade/src/core/desktop.rs:180` |
| 6b | Notification Service | Service manager initializes notification subsystem | `ade/src/service/service_manager.rs:18` |
| 6c | Session Service | Session manager tracks boot time and login state | `ade/src/service/service_manager.rs:20` |
| 6d | Service Tick Dispatch | Desktop tick propagates to all managed services | `ade/src/core/desktop.rs:273` |
| 6e | Notification Tick | Notification manager processes timeouts and dismissals | `ade/src/service/service_manager.rs:26` |
| 6f | Timeout Check | Notification service auto-dismisses expired notifications | `ade/src/service/notification.rs:102` |

### Code Snippets

**File: ade/src/core/desktop.rs (Lines 178-182)**
```rust
            app_reg: crate::util::app_registry::AppRegistry::new(),
            lifecycle: crate::sys::lifecycle::LifecycleManager::new(),
            services: crate::service::service_manager::ServiceManager::new(0),
            tray: SystemTray::new(),
            settings: crate::core::settings::SettingsState::new(),
```

**File: ade/src/service/service_manager.rs (Lines 16-22)**
```rust
    pub fn new(boot_tick: u64) -> Self {
        ServiceManager {
            notifications: NotificationManager::new(),
            clipboard: ClipboardManager::new(),
            session: SessionManager::new(boot_tick),
            power: PowerManager::new(),
        }
```

**File: ade/src/service/service_manager.rs (Lines 24-28)**
```rust

    pub fn tick(&mut self, current_tick: u64) {
        self.notifications.tick(current_tick);
        self.power.tick(current_tick);
    }
```

**File: ade/src/service/notification.rs (Lines 100-104)**
```rust
        while i < self.notifications.len() {
            if self.notifications[i].timeout > 0 && !self.notifications[i].di...
                if current_tick >= self.notifications[i].created_tick + self....
                    self.notifications[i].dismissed = true;
                    self.visible_count = self.visible_count.saturating_sub(1);
```

---

## Trace 7: Accessibility Tree: Build and Focus Navigation

**Description:** Security/accessibility layer - shows a11y tree construction from desktop state, node hierarchy, and keyboard focus management

### Flow Diagram

```
Desktop Tick Cycle
├── desktop.tick() per-frame update <-- desktop.rs:234
│   ├── build_a11y_tree() <-- 7a
│   │   ├── a11y_tree.clear() <-- 7b
│   │   ├── Desktop root node
│   │   │   └── add_node(Desktop, ...) <-- 7c
│   │   │       └── nodes.push(A11yNode) <-- 7d
│   │   ├── Taskbar hierarchy
│   │   │   └── add_child(desktop, taskbar) <-- 7e
│   │   ├── Window nodes (loop) <-- desktop.rs:377
│   │   └── Desktop icon nodes (loop) <-- desktop.rs:397
│   └── handle_a11y_key(key) <-- desktop.rs:601
│       └── focus.move_focus(dir, tree) <-- 7f
│           └── focus(node.id) <-- 7g
└── Event dispatch
    └── handle_event(Key) <-- desktop.rs:538
        └── handle_a11y_key() <-- desktop.rs:539
```

### Locations

| ID | Title | Description | Path:LineNumber |
|----|-------|-------------|-----------------|
| 7a | Tree Build Trigger | Desktop tick rebuilds accessibility tree from current state | `ade/src/core/desktop.rs:275` |
| 7b | Tree Reset | Accessibility tree cleared for fresh rebuild each frame | `ade/src/core/desktop.rs:321` |
| 7c | Root Node Creation | Desktop node added as root of accessibility hierarchy | `ade/src/core/desktop.rs:325` |
| 7d | Node Registration | A11y tree stores node with role, bounds, and focus state | `ade/src/sec/a11y/tree.rs:35` |
| 7e | Hierarchy Link | Parent-child relationship establishes tree structure | `ade/src/core/desktop.rs:339` |
| 7f | Focus Navigation | Keyboard navigation moves focus through accessible nodes | `ade/src/core/desktop.rs:611` |
| 7g | Focus Assignment | FocusManager updates focused node and history | `ade/src/sec/a11y/focus.rs:47` |

### Code Snippets

**File: ade/src/core/desktop.rs (Lines 271-277)**
```rust
            self.damage.mark_full();
        }
        self.services.tick(self.clock_ticks);
        self.watcher.poll();
        self.build_a11y_tree();
        self.tooltips.tick();
        self.tick_tooltip_hover();
```

**File: ade/src/core/desktop.rs (Lines 319-327)**
```rust

    fn build_a11y_tree(&mut self) {
        self.a11y_tree.clear();
        let ty = self.taskbar_y();

        // root: Desktop
        let desktop_id = self.a11y_tree.add_node(
            crate::sec::a11y::A11yRole::Desktop,
            "Desktop",
```

**File: ade/src/core/desktop.rs (Lines 337-341)**
```rust
            true,
        );
        self.a11y_tree.add_child(desktop_id, taskbar_id);

        // Start button
```

**File: ade/src/core/desktop.rs (Lines 609-613)**
```rust
                    crate::sec::a11y::FocusDirection::Down
                };
                self.focus.move_focus(dir, &self.a11y_tree);
                self.damage.mark_full();
                true
```

**File: ade/src/sec/a11y/tree.rs (Lines 33-37)**
```rust
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.push(A11yNode {
            id,
            role,
```

**File: ade/src/sec/a11y/focus.rs (Lines 45-49)**
```rust
                for i in start..tree.nodes.len() {
                    if tree.nodes[i].focusable && tree.nodes[i].state.visible {
                        self.focus(tree.nodes[i].id);
                        return true;
                    }
```

---

## Trace 8: Lifecycle Management: Process Tracking and Reaping

**Description:** System lifecycle service - demonstrates process registration, state tracking, crash detection, and zombie process cleanup

### Flow Diagram

```
Desktop Event Loop (main.rs)
├── tick() frame update <-- 8c
│   └── process::waitpid(-1, 1) reap children <-- desktop.rs:223
│       ├── match Ok((pid, status)) if pid > 0 <-- desktop.rs:224
│       │   ├── exit_class(status): 0=Clean, 1..127=Error, 128+sig=Signal, <0=Killed
│       │   │   ├── Clean → lifecycle.mark_terminated(pid) <-- 8d
│       │   │   └── else → lifecycle.mark_crashed(pid) + crash notification <-- 8f
│       │   ├── lifecycle.remove(pid) — no unbounded proc accumulation
│       │   ├── permissions.unregister(pid) — grant revoked on every exit
│       │   └── wm.close_by_pid(pid) <-- 8e
│       │       └── WindowManager::close_by_pid() <-- window_manager.rs:62
│       │           └── windows.remove(pos) <-- window_manager.rs:64
│       └── else break

Application Launch Flow (launcher.rs)
└── spawn_app_at() fork success path <-- launcher.rs:99
    ├── lifecycle.register(pid, app_idx) <-- 8a
    │   └── LifecycleManager::register() <-- lifecycle.rs:44
    │       └── procs.push(AppLifecycle) state: Starting <-- lifecycle.rs:45
    ├── permissions.register(pid, default_grant()) — everyday caps, no power
    ├── lifecycle.mark_running(pid) → Running <-- 8b
    └── window + launch notification
```

### Locations

| ID | Title | Description | Path:LineNumber |
|----|-------|-------------|-----------------|
| 8a | Process Registration | LifecycleManager tracks new process with restart policy | `ade/src/sys/lifecycle.rs:45` |
| 8b | State Transition | Process marked as running after successful startup | `ade/src/sys/lifecycle.rs:58` |
| 8c | Child Reaping | Desktop tick reaps terminated child processes non-blocking | `ade/src/core/desktop.rs:223` |
| 8d | Termination Mark | Clean exits (wait status 0) marked terminated | `ade/src/core/desktop.rs:229` |
| 8e | Window Cleanup | Window manager removes windows associated with dead process | `ade/src/core/desktop.rs:244` |
| 8f | Crash Detection | Non-clean exits classified (Error/Signal/Killed) and marked crashed | `ade/src/sys/lifecycle.rs:27` |
| 8g | Crash Counter | Crash count incremented for restart throttling logic | `ade/src/sys/lifecycle.rs:97` |

### Code Snippets

**File: ade/src/core/desktop.rs (Lines 225-247)**
```rust
    pub fn reap_children(&mut self) {
        loop {
            match process::waitpid(-1, 1) {
                Ok((pid, status)) if pid > 0 => {
                    use crate::sys::lifecycle::ExitClass;
                    match crate::sys::lifecycle::exit_class(status) {
                        ExitClass::Clean => self.lifecycle.mark_terminated(pid),
                        cls => {
                            self.lifecycle.mark_crashed(pid);
                            let reason = match cls {
                                ExitClass::Killed => alloc::string::String::from("killed"),
                                ExitClass::Signal(sig) => alloc::format!("signal {}", sig),
                                ExitClass::Error(code) => alloc::format!("exit {}", code),
                                ExitClass::Clean => unreachable!(),
                            };
                            self.services
                                .notify("Application Crashed", &reason, 2, 8000);
                        }
                    }
                    self.lifecycle.remove(pid);
                    self.permissions.unregister(pid);
                    self.wm.close_by_pid(pid);
                    self.damage.mark_full();
                }
                _ => break,
            }
        }
    }
```

**File: ade/src/sys/lifecycle.rs (Lines 43-47)**
```rust

    pub fn register(&mut self, pid: u64, app_idx: usize) {
        self.procs.push(AppLifecycle {
            pid,
            state: AppState::Starting,
```

**File: ade/src/sys/lifecycle.rs (Lines 56-60)**
```rust
        for p in &mut self.procs {
            if p.pid == pid && p.state == AppState::Starting {
                p.state = AppState::Running;
                return;
            }
```

**File: ade/src/sys/lifecycle.rs (Lines 75-80)**
```rust
        for p in &mut self.procs {
            if p.pid == pid {
                p.state = AppState::Crashed;
                p.crash_count += 1;
                return;
            }
```

---

## Summary

The ADE Desktop Environment architecture demonstrates a clean separation of concerns across several core subsystems:

1. **Application Launch** - Event-driven process spawning with lifecycle tracking
2. **Window Management** - Z-ordered window collection with focus and animation
3. **IPC Services** - Portal-based request routing with permission validation (real transport: AF_UNIX socketpair, see Trace 3)
4. **Security** - Bitwise permission sets with per-process registration
5. **Rendering** - Damage-based composition with layer-based alpha blending
6. **System Services** - Centralized service manager with per-frame tick propagation
7. **Accessibility** - Tree-based a11y representation with keyboard navigation
8. **Lifecycle** - Process tracking with crash detection and restart policies

All components integrate through the main event loop in `main.rs`, which coordinates input handling, state updates, damage tracking, and frame rendering at approximately 60Hz.
