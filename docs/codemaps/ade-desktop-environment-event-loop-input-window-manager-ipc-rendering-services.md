# ADE Desktop Environment: Event Loop, Input, Window Manager, IPC, Rendering & Services

**Codemap ID:** ADE_Desktop_Environment__Event_Loop__Input__Window_Manager__IPC__Rendering___Services_20260731_155018

**Description:** End-to-end runtime flow of the ADE desktop environment from event polling to rendering, covering input handling, window management, IPC permission enforcement, service dispatch, and compositor pipeline. Notable breakage points: no event queue [1c], full-frame compositor recomposite [5e], dual permission systems [4d], and manual process reaping [6b].

---

## Trace 1: Main Event Loop: Poll → Dispatch → Render → Sleep

**Description:** Core event loop in main.rs that drives the entire desktop, polling kernel for input and orchestrating the frame cycle

### Trace Diagram

```
ADE Main Event Loop (main.rs)
└── user_main() entrypoint <-- main.rs:21
    └── while running { <-- 1a
        ├── desktop.tick() <-- 1b
        │   ├── advance_clock() <-- desktop.rs:222
        │   ├── reap_children() <-- desktop.rs:272
        │   ├── process_ipc() <-- desktop.rs:277
        │   └── services.tick() <-- desktop.rs:301
        ├── Input Polling
        │   ├── while let Some(key) = get_key() <-- 1c
        │   │   └── desktop.handle_event(Key) <-- 1d
        │   └── let ms = get_mouse() <-- 1e
        │       └── desktop.handle_event(Click) <-- 1f
        ├── Rendering
        │   ├── if damage.is_dirty() <-- main.rs:78
        │   │   ├── desktop.snapshot() <-- main.rs:80
        │   │   └── render::render() <-- 1g
        │   │       └── compositor.compose() <-- mod.rs:208
        │   │           └── window.flush() <-- main.rs:82
        │   └── damage.clear() <-- main.rs:85
        └── syscall2(35, 0, sleep_ns) <-- 1h
```

### Location Details

**Location ID 1a: Event loop start**
- **Description:** Main loop that runs until user exits desktop
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:45

**Location ID 1b: Frame tick**
- **Description:** Advances clock, reaps children, processes IPC, ticks services and animations
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:46

**Location ID 1c: Poll keyboard**
- **Description:** Syscall to kernel - no event queue, polls until empty
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:48

**Location ID 1d: Dispatch key event**
- **Description:** Direct dispatch to Desktop coordinator, no queuing
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:55

**Location ID 1e: Poll mouse state**
- **Description:** Syscall to kernel for mouse position and buttons
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:59

**Location ID 1f: Dispatch mouse click**
- **Description:** Mouse events dispatched immediately after detection
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:66

**Location ID 1g: Render frame**
- **Description:** Compositor blends 6 layers to framebuffer if damage is dirty
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:81

**Location ID 1h: Sleep syscall**
- **Description:** Adaptive frame pacing - sleeps to target 60fps
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:98

---

## Trace 2: Mouse Click Flow: Input → Hit Test → Window Focus → Damage

**Description:** User interaction path from mouse click through Desktop coordinator to window manager state changes

### Trace Diagram

```
Mouse Click Flow (Trace 2)
│
├── Kernel Layer
│   └── Syscall 120: get_mouse state <-- 2b
│
├── libsarga (GUI Library)
│   └── Window::get_mouse() wrapper <-- 2a
│       └── calls syscall, unpacks state <-- io.rs:565
│
├── Main Event Loop (main.rs)
│   └── desktop_win.get_mouse() <-- main.rs:59
│       └── desktop.update_mouse() <-- main.rs:60
│           └── detects click, emits Event <-- main.rs:66
│
├── Desktop Coordinator
│   ├── handle_event(MouseClick) <-- 2c
│   │   └── match on Event variant <-- desktop.rs:565
│   └── handle_click(x, y) <-- 2d
│       └── hit test windows, taskbar, icons <-- desktop.rs:1368
│           └── window hit: bring_to_front() <-- 2e
│
├── Window Manager
│   ├── bring_to_front(WindowId) <-- 2e
│   │   ├── windows.remove(id) <-- 2f
│   │   │   └── Vec remove at index
│   │   └── windows.push(w) <-- 2g
│   │       └── last = topmost z-order
│   └── focused = Some(last_index) <-- window_manager.rs:111
│
└── Damage Tracker
    └── mark_full() <-- 2h
        └── triggers full-frame recomposite
```

### Location Details

**Location ID 2a: Window get_mouse**
- **Description:** libsarga Window wrapper calls kernel syscall 120
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\gui.rs:869

**Location ID 2b: Syscall wrapper**
- **Description:** Unpacks kernel's packed mouse state (x, y, buttons, scroll)
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\io.rs:563

**Location ID 2c: Event dispatch**
- **Description:** Desktop.handle_event matches on MouseClick variant
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:571

**Location ID 2d: Click handler**
- **Description:** Delegates to handle_click for hit testing and state updates
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:573

**Location ID 2e: Focus window**
- **Description:** Window manager removes window from Vec and pushes to end
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1380

**Location ID 2f: Remove from list**
- **Description:** Vec-based window storage - remove and re-add to reorder
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\window_manager.rs:105

**Location ID 2g: Push to front**
- **Description:** Last window in Vec is topmost - simple z-order
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\window_manager.rs:110

**Location ID 2h: Mark damage**
- **Description:** Sets dirty flag to trigger full-frame recomposite
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1166

---

## Trace 3: App Launch: Fork → Exec → IPC Setup → Window Creation → Permission Grant

**Description:** Complete lifecycle of launching an external application from the launcher subsystem through process creation to IPC registration

### Trace Diagram

```
App Launch Flow (Trace 3)
├── Desktop coordinator receives launch request
│   └── spawn_app_from_registry() <-- 3a
│       └── spawn_app_at() <-- launcher.rs:50
│           ├── Create IPC channel
│           │   └── socketpair(AF_UNIX) <-- 3b
│           ├── Fork process
│           │   └── fork() syscall <-- 3c
│           │       ├── Child branch (pid == 0) <-- launcher.rs:101
│           │       │   └── execve(path, argv) <-- 3d
│           │       └── Parent branch (pid > 0) <-- launcher.rs:115
│           │           ├── Grant permissions <-- 3e
│           │           ├── Register IPC fd <-- 3f
│           │           └── Create window <-- 3g
│           └── Mark damage & notify <-- launcher.rs:148
```

### Location Details

**Location ID 3a: Launch from registry**
- **Description:** Start menu or icon click triggers app launch
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:875

**Location ID 3b: Create IPC socketpair**
- **Description:** AF_UNIX socketpair for parent-child IPC channel
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:94

**Location ID 3c: Fork process**
- **Description:** Syscall to kernel - creates child process
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:100

**Location ID 3d: Exec in child**
- **Description:** Child process replaces itself with app binary, passes IPC fd
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:107

**Location ID 3e: Grant permissions**
- **Description:** Parent grants default permissions (clipboard, notifications, filesystem, window, settings)
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:123

**Location ID 3f: Register IPC transport**
- **Description:** Maps pid to server-side socket fd for future IPC
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:127

**Location ID 3g: Create window**
- **Description:** Window manager adds AppWindow to Vec, sets focus
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:142

---

## Trace 4: IPC Request: Transport Poll → Permission Check → Portal Dispatch → Service

**Description:** External app IPC request flow through transport layer, permission enforcement, and portal routing to service handlers

### Trace Diagram

```
Desktop Event Loop (main.rs)
└── while running <-- main.rs:45
    └── desktop.tick() <-- 4a
        ├── ipc_transport.ingest() poll sockets <-- desktop.rs:273
        │   └── libsarga::net::poll(fds, 0) <-- 4b
        │       └── returns Vec<ServiceRequest>
        ├── ipc_server.submit_request(req) <-- desktop.rs:275
        └── process_ipc() drain & dispatch <-- 4c
            ├── permissions.granted(pid) lookup <-- 4d
            ├── service_registry.find(service_id) <-- desktop.rs:1757
            │   └── check required_permissions <-- desktop.rs:1761
            └── if allowed <-- desktop.rs:1765
                ├── portal::dispatch(desktop, app, req) <-- 4e
                │   └── match service_id <-- 4f
                │       ├── Clipboard → clipboard::handle_request() <-- mod.rs:12
                │       ├── Notification → notification::handle_request() <-- mod.rs:13
                │       └── ... → service handler
                │           └── returns ServiceResponse
                ├── ipc_server.submit_response(resp) <-- 4g
                └── ipc_transport.deliver(responses) <-- 4h
                    └── write_frame(fd, response) <-- transport.rs:116
```

### Location Details

**Location ID 4a: Poll IPC transport**
- **Description:** Desktop.tick() polls all registered socketpairs with timeout 0
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:273

**Location ID 4b: Poll syscall**
- **Description:** Non-blocking poll on all peer fds, reads frames if ready
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\ipc\transport.rs:60

**Location ID 4c: Process IPC batch**
- **Description:** Drains up to 64 requests per frame to avoid stalling
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:277

**Location ID 4d: Check permissions**
- **Description:** PermissionManager looks up pid's granted permission bitmask
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1755

**Location ID 4e: Portal dispatch**
- **Description:** If allowed, routes to portal handler by ServiceId
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1766

**Location ID 4f: Route to handler**
- **Description:** Portal matches ServiceId and calls appropriate handler
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\sec\portal\mod.rs:12

**Location ID 4g: Queue response**
- **Description:** Response added to pending queue for delivery
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1775

**Location ID 4h: Deliver responses**
- **Description:** Writes response frames back to peer sockets
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:279

---

## Trace 5: Render Pipeline: Snapshot → Layer Drawing → Alpha Blend → Flush

**Description:** Frame rendering flow from state capture through 6-layer compositor to kernel framebuffer flush

### Trace Diagram

```
ADE Render Pipeline (Trace 5)
├── main.rs event loop <-- main.rs:45
│   ├── if desktop.damage.is_dirty() <-- 5a
│   ├── desktop.snapshot() <-- 5b
│   ├── render::render() call <-- main.rs:81
│   │   ├── comp.clear_all() <-- 5c
│   │   ├── Layer drawing phase
│   │   │   ├── draw wallpaper <-- mod.rs:25
│   │   │   ├── for aw in snap.windows <-- 5d
│   │   │   │   └── window::draw() <-- mod.rs:53
│   │   │   ├── draw taskbar <-- mod.rs:68
│   │   │   └── draw cursor <-- mod.rs:173
│   │   └── comp.compose() call <-- mod.rs:208
│   │       ├── dst.copy_from_slice(wallpaper) <-- 5e
│   │       └── for each layer 1..6 <-- compositor.rs:792
│   │           └── alpha_blend pixels <-- 5f
│   ├── desktop_win.flush() <-- 5g
│   │   └── syscall2(SYS_GUI_FLUSH) <-- gui.rs:475
│   └── desktop.damage.clear() <-- 5h
└── Result: framebuffer updated in kernel
```

### Location Details

**Location ID 5a: Check damage**
- **Description:** Only render if damage tracker marked dirty this frame
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:78

**Location ID 5b: Capture snapshot**
- **Description:** Read-only borrows of all subsystem state for rendering
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:80

**Location ID 5c: Clear layers**
- **Description:** Zeros all 6 screen-sized pixel buffers (18MB at 1024x768)
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs:20

**Location ID 5d: Draw windows**
- **Description:** Iterates windows and draws to Layer::Windows canvas
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs:48

**Location ID 5e: Copy wallpaper**
- **Description:** Full-frame compose: starts with wallpaper base layer
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs:789

**Location ID 5f: Alpha blend pixel**
- **Description:** Blends each layer pixel over accumulated output - 3.9M ops per frame
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs:802

**Location ID 5g: Flush to kernel**
- **Description:** Syscall 102 copies framebuffer to kernel display driver
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:82

**Location ID 5h: Clear damage**
- **Description:** Resets dirty flag until next state change
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:85

---

## Trace 6: Process Exit: Waitpid → Lifecycle Cleanup → IPC Unregister → Window Close

**Description:** Child process termination handling through reaping, resource cleanup, and window removal

### Trace Diagram

```
Desktop Event Loop (per frame)
└── Desktop.tick() <-- 6a
    └── Desktop.reap_children() <-- desktop.rs:225
        └── loop <-- desktop.rs:226
            ├── process::waitpid(-1, 1) <-- 6b
            ├── exit_class(status) <-- 6c
            │   └── classify as Clean/Error/Signal/Killed
            ├── lifecycle.mark_terminated(pid) <-- desktop.rs:231
            ├── lifecycle.remove(pid) <-- desktop.rs:244
            ├── permissions.unregister(pid) <-- 6d
            ├── ipc_transport.unregister(pid) <-- 6e
            │   └── close server_fd & remove from peers <-- transport.rs:37
            ├── wm.close_by_pid(pid) <-- 6f
            │   └── windows.remove(pos) <-- 6g
            └── damage.mark_full() <-- desktop.rs:248
```

### Location Details

**Location ID 6a: Reap children**
- **Description:** Called every tick to collect zombie processes
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:272

**Location ID 6b: Waitpid syscall**
- **Description:** Non-blocking wait for any child, loops until no more zombies
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:227

**Location ID 6c: Classify exit**
- **Description:** Determines if clean exit, error, signal, or killed
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\sys\lifecycle.rs:28

**Location ID 6d: Revoke permissions**
- **Description:** Removes pid from permission table
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:245

**Location ID 6e: Close IPC socket**
- **Description:** Closes server-side fd and removes from peer list
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:246

**Location ID 6f: Close window**
- **Description:** Removes window from Vec by pid match
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:247

**Location ID 6g: Remove from list**
- **Description:** Vec::remove shifts all subsequent windows down
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\window_manager.rs:64

---

## Trace 7: Start Menu Interaction: Click → Filter Rebuild → App Launch

**Description:** User opens start menu, searches/selects app, and triggers launch flow

### Trace Diagram

```
Start Menu Interaction Flow
├── Taskbar Click Handler
│   └── handle_click(mx, my) <-- 7a
│       └── start_menu.open_with(&app_reg) <-- desktop.rs:1320
│           └── rebuild_filter(reg) <-- 7b
│               └── filtered = reg.filtered(cat, search) <-- start_menu.rs:77
│
├── Keyboard Input in Start Menu
│   └── handle_key(key) <-- desktop.rs:879
│       └── if start_menu.open <-- desktop.rs:900
│           └── search.push(ch) <-- 7c
│               └── rebuild_filter(&app_reg) <-- desktop.rs:932
│
└── App List Click Handler
    └── handle_click(mx, my) <-- desktop.rs:1165
        └── if mx in list_x..list_w <-- desktop.rs:1238
            ├── app_id = filtered[i] <-- 7d
            └── launch_app(app_id) <-- 7e
                ├── start_menu.open = false <-- 7f
                ├── spawn_app_from_registry(self, &app) <-- desktop.rs:875
                └── damage.mark_full() <-- desktop.rs:876
```

### Location Details

**Location ID 7a: Open start menu**
- **Description:** Taskbar button click sets open flag and rebuilds filter
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1320

**Location ID 7b: Rebuild filter**
- **Description:** Filters app registry by category and search text
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\start_menu.rs:71

**Location ID 7c: Append search char**
- **Description:** Keyboard input in start menu updates search buffer
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:929

**Location ID 7d: Get selected app**
- **Description:** Click on app list item retrieves AppId from filtered list
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1239

**Location ID 7e: Launch app**
- **Description:** Delegates to launch_app which calls spawn_app_from_registry
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1240

**Location ID 7f: Close menu**
- **Description:** Launch closes start menu and marks damage for re-render
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:844

---

## Trace 8: Accessibility Tree Build: Tick → Clear → Add Nodes → Sync Focus

**Description:** Per-frame accessibility tree reconstruction for screen readers and keyboard navigation

### Trace Diagram

```
Desktop Frame Tick (every 16ms)
└── desktop.tick() <-- 1b
    └── build_a11y_tree() <-- 8a
        ├── a11y_tree.clear() <-- 8b
        │   ├── nodes.clear() <-- tree.rs:21
        │   └── focused_id = None <-- tree.rs:22
        ├── Add root desktop node <-- 8c
        │   └── a11y_tree.add_node(Desktop, bounds) <-- desktop.rs:353
        ├── Add taskbar + start button <-- desktop.rs:361
        ├── Iterate windows for taskbar btns <-- 8d
        │   └── for i in 0..wm.len() <-- desktop.rs:379
        │       └── add_node(Button, title, bounds) <-- desktop.rs:382
        ├── Add start menu (if open) <-- desktop.rs:392
        ├── Add window nodes with close buttons <-- desktop.rs:405
        ├── Add desktop icons <-- 8e
        │   └── for ic in desktop_icons.icons <-- desktop.rs:425
        │       └── add_node(Icon, name, bounds) <-- desktop.rs:426
        ├── Add notifications <-- desktop.rs:436
        └── Sync focus from FocusManager <-- 8f
            └── a11y_tree.set_focus(focused_id) <-- desktop.rs:448

Keyboard Navigation (arrow keys)
└── FocusManager.move_focus() <-- 8g
    ├── Match direction (Up/Down/Left/Right) <-- focus.rs:40
    ├── Find nearest focusable node <-- focus.rs:110
    └── self.focus(node_id) <-- focus.rs:134
```

### Location Details

**Location ID 8a: Build a11y tree**
- **Description:** Called every tick to rebuild accessibility tree from scratch
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:303

**Location ID 8b: Clear tree**
- **Description:** Removes all nodes and resets ID counter - full rebuild
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:349

**Location ID 8c: Add desktop root**
- **Description:** Creates root node with Desktop role and screen bounds
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:353

**Location ID 8d: Add taskbar buttons**
- **Description:** Iterates windows to create taskbar button nodes
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:379

**Location ID 8e: Add desktop icons**
- **Description:** Creates Icon role nodes for each desktop icon
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:425

**Location ID 8f: Sync focus**
- **Description:** Copies FocusManager state to a11y tree focused_id
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:447

**Location ID 8g: Move focus**
- **Description:** Arrow key navigation uses spatial or sequential focus movement
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\sec\a11y\focus.rs:39

---

## Trace 9: Service Tick: Notification Timeout → Dismiss → Damage

**Description:** Per-frame service maintenance including notification expiration and cleanup

### Trace Diagram

```
Desktop Frame Tick (every ~16ms)
└── desktop.tick() <-- 9a
    └── services.tick(clock_ticks) <-- desktop.rs:301
        └── ServiceManager::tick() <-- service_manager.rs:25
            ├── notifications.tick() <-- 9b
            │   └── NotificationManager::tick() <-- notification.rs:98
            │       └── for each notification <-- notification.rs:100
            │           ├── if timeout expired <-- 9c
            │           ├── mark dismissed <-- 9d
            │           ├── decrement visible_count <-- notification.rs:104
            │           └── swap to end of Vec <-- 9e
            └── power.tick() <-- service_manager.rs:27
                └── PowerManager::tick()
```

### Location Details

**Location ID 9a: Tick services**
- **Description:** ServiceManager ticks all owned services with current time
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:301

**Location ID 9b: Tick notifications**
- **Description:** Delegates to NotificationManager for timeout processing
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\service\service_manager.rs:26

**Location ID 9c: Check timeout**
- **Description:** Compares current tick against notification creation + timeout
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\service\notification.rs:102

**Location ID 9d: Mark dismissed**
- **Description:** Sets dismissed flag and decrements visible_count
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\service\notification.rs:103

**Location ID 9e: Swap to end**
- **Description:** Keeps visible notifications contiguous at start of Vec
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\service\notification.rs:107

---

## Code Snippets

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\window_manager.rs

```rust
Lines: 62-66
    pub fn close_by_pid(&mut self, pid: u64) {
        if let Some(pos) = self.windows.iter().position(|w| w.pid == Some(pid...
            self.windows.remove(pos);
            if self.windows.is_empty() {
                self.focused = None;
```

```rust
Lines: 103-112
            return;
        }
        let mut w = self.windows.remove(id.0);
        w.focused = true;
        for other in &mut self.windows {
            other.focused = false;
        }
        self.windows.push(w);
        self.focused = Some(self.windows.len() - 1);
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\sec\portal\mod.rs

```rust
Lines: 10-14
pub(crate) fn dispatch(desktop: &mut Desktop, app: ApplicationId, req: &Servi...
    match req.service {
        crate::ipc::ServiceId::Clipboard => clipboard::handle_request(desktop...
        crate::ipc::ServiceId::Notification => notification::handle_request(d...
        crate::ipc::ServiceId::Settings => settings::handle_request(desktop, ...
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs

```rust
Lines: 18-22
    comp: &mut Compositor,
) {
    comp.clear_all();

    // Wallpaper
```

```rust
Lines: 46-50
    {
        let mut cv = comp.layer_canvas(Layer::Windows);
        for aw in snap.windows {
            if !aw.always_on_top {
                if aw.flags.shadow {
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\service\service_manager.rs

```rust
Lines: 24-28

    pub fn tick(&mut self, current_tick: u64) {
        self.notifications.tick(current_tick);
        self.power.tick(current_tick);
    }
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs

```rust
Lines: 43-50
    let mut running = true;
    let mut last_frame_ticks = 0u64;
    while running {
        desktop.tick();

        while let Some(key) = desktop_win.get_key() {
            // Session lifecycle: Ctrl+Alt+Backspace → clean session end
            if key == 0x7F || key == 0x08 {
```

```rust
Lines: 53-61
                break;
            }
            desktop.handle_event(core::event::Event::Key(key));
        }
        if !running { break; }

        let ms = desktop_win.get_mouse();
        let (pressed, released, dragging) =
            desktop.update_mouse(ms.x as i32, ms.y as i32, ms.buttons & 1 != ...
```

```rust
Lines: 64-68
        }
        if pressed {
            desktop.handle_event(core::event::Event::MouseClick(ms.x as i32, ...
        } else if ms.buttons & 4 != 0 {
            desktop.handle_event(core::event::Event::MouseMiddle(ms.x as i32,...
```

```rust
Lines: 79-84
            let clock_str = desktop.prepare_clock();
            let snap = desktop.snapshot();
            render::render(&mut desktop_win, &snap, &clock_str, &mut composit...
            if let Err(e) = desktop_win.flush() {
                io::print_str(&alloc::format!("[ade] flush error: {}\n", e));
            }
```

```rust
Lines: 96-100
        };
        unsafe {
            libsarga::syscall::syscall2(35, 0, sleep_ns);
        }
    }
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\gui.rs

```rust
Lines: 867-871
    }

    pub fn get_mouse(&self) -> crate::io::MouseState {
        crate::io::get_mouse(self.id)
    }
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\start_menu.rs

```rust
Lines: 69-73
    }

    pub fn rebuild_filter(&mut self, reg: &AppRegistry) {
        let cat = if self.cat_idx < CATEGORIES.len() {
            CATEGORIES[self.cat_idx].1
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\service\notification.rs

```rust
Lines: 100-109
        while i < self.notifications.len() {
            if self.notifications[i].timeout > 0 && !self.notifications[i].di...
                if current_tick >= self.notifications[i].created_tick + self....
                    self.notifications[i].dismissed = true;
                    self.visible_count = self.visible_count.saturating_sub(1);
                    // Swap to keep visible contiguous
                    if i < self.visible_count {
                        self.notifications.swap(i, self.visible_count);
                    }
                }
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs

```rust
Lines: 76-87
    pub(crate) screen_h: u32,
    pub(crate) wm: WindowManager,
    pub(crate) start_menu: StartMenuState,
    pub(crate) context_menu: Option<(i32, i32, &'static [(&'static str, &'sta...
    pub(crate) clock_ticks: u64,
    pub(crate) mouse_x: i32,
    pub(crate) mouse_y: i32,
    mouse_btn: bool,
    prev_mouse_btn: bool,
    pub(crate) drag_active: bool,
    pub(crate) cursor: Cursor,
    pub(crate) cursor_visible: bool,
```

```rust
Lines: 225-229
    pub fn reap_children(&mut self) {
        loop {
            match process::waitpid(-1, 1) {
                Ok((pid, status)) if pid > 0 => {
                    use crate::sys::lifecycle::ExitClass;
```

```rust
Lines: 243-249
                    }
                    self.lifecycle.remove(pid);
                    self.permissions.unregister(pid);
                    self.ipc_transport.unregister(pid);
                    self.wm.close_by_pid(pid);
                    self.damage.mark_full();
```

```rust
Lines: 270-281
            self.damage.mark_full();
        }
        self.reap_children();
        let reqs = self.ipc_transport.ingest();
        for req in reqs {
            self.ipc_server.submit_request(req);
        }
        self.process_ipc();
        let responses = self.ipc_server.drain_responses();
        self.ipc_transport.deliver(responses);
        let mut anim_active = false;
        for w in self.wm.iter_mut() {
```

```rust
Lines: 299-305
            self.damage.mark_full();
        }
        self.services.tick(self.clock_ticks);
        self.watcher.poll();
        self.build_a11y_tree();
        self.tooltips.tick();
        self.tick_tooltip_hover();
```

```rust
Lines: 347-355

    fn build_a11y_tree(&mut self) {
        self.a11y_tree.clear();
        let ty = self.taskbar_y();

        // root: Desktop
        let desktop_id = self.a11y_tree.add_node(
            crate::sec::a11y::A11yRole::Desktop,
            "Desktop",
```

```rust
Lines: 377-381

        // Window buttons in taskbar
        for i in 0..self.wm.len() {
            let aw = &self.wm.iter()[i];
            let bx = 75 + i as u32 * 125;
```

```rust
Lines: 423-427

        // Desktop Icons
        for ic in &self.desktop_icons.icons {
            let icon_id = self.a11y_tree.add_node(
                crate::sec::a11y::A11yRole::Icon,
```

```rust
Lines: 445-449

        // Sync focus from FocusManager
        if let Some(fid) = self.focus.focused() {
            self.a11y_tree.set_focus(fid);
        }
```

```rust
Lines: 569-575
                }
            }
            Event::MouseClick(x, y) => {
                self.focus_visible = false;
                self.handle_click(x, y);
            }
            Event::MouseMiddle(x, y) => {
```

```rust
Lines: 842-846

    fn launch_app(&mut self, app_id: AppId) {
        self.start_menu.open = false;
        let app = match self.app_reg.get(app_id) {
            Some(a) => *a,
```

```rust
Lines: 873-877
            return;
        }
        crate::core::launcher::spawn_app_from_registry(self, &app);
        self.damage.mark_full();
```

```rust
Lines: 927-931
                    self.damage.mark_full();
                }
                ch if (ch >= 0x20 && ch <= 0x7E) => {
                    // printable ASCII → search
                    self.start_menu.search.push(ch);
```

```rust
Lines: 1164-1168

    pub(crate) fn handle_click(&mut self, mx: i32, my: i32) {
        self.damage.mark_full();
        if self.settings.open {
            if let Some(idx) = self.settings.hit_test(mx, my, &self.snapshot(...
```

```rust
Lines: 1237-1242
                let iy = list_y + 2 + (i - start) as i32 * 32;
                if mx >= list_x && mx < list_x + list_w && my >= iy && my < i...
                    let app_id = self.start_menu.filtered[i];
                    self.launch_app(app_id);
                    return;
```

```rust
Lines: 1318-1322
        if my >= taskbar_y {
            if mx >= 5 && mx < 65 {
                self.start_menu.open_with(&self.app_reg);
                return;
            }
```

```rust
Lines: 1378-1382
                    return;
                }
                self.wm.bring_to_front(WindowId(i));
                self.wm.begin_drag(WindowId(i), mx, my);
                return;
```

```rust
Lines: 1753-1757
        for req in requests {
            let app = req.sender;
            let granted = self.permissions.granted(app.0);
            let allowed = self
                .service_registry
```

```rust
Lines: 1764-1768
                .unwrap_or(false);
            let resp = if allowed {
                crate::sec::portal::dispatch(self, app, &req)
            } else {
                crate::ipc::ServiceResponse {
```

```rust
Lines: 1773-1777
                }
            };
            self.ipc_server.submit_response(resp);
        }
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs

```rust
Lines: 92-96

    if !path.is_empty() {
        let ipc_pair = libsarga::net::socketpair(
            libsarga::net::SocketDomain::Unix as u64,
            libsarga::net::SocketType::Stream as u64,
```

```rust
Lines: 98-102
        )
        .ok();
        match libsarga::process::fork() {
            Ok(0) => {
                match ipc_pair {
```

```rust
Lines: 105-109
                        let fd_arg = alloc::format!("{}", client_fd);
                        let argv = [path, "--ipc-fd", fd_arg.as_str()];
                        let _ = libsarga::process::execve(path, &argv, &[]);
                    }
                    None => {
```

```rust
Lines: 121-129
                    .unwrap_or(0);
                desktop.lifecycle.register(pid, app_idx);
                desktop.permissions.register(pid, crate::sec::perms::default_...
                desktop.lifecycle.mark_running(pid);
                if let Some((server_fd, client_fd)) = ipc_pair {
                    let _ = libsarga::io::close(client_fd);
                    desktop.ipc_transport.register(pid, server_fd);
                }
                app_win
```

```rust
Lines: 140-144
        }
    }
    let id = desktop.wm.create(app_win);
    if let Some(w) = self.wm.lookup_mut(id) {
        w.flags.opacity = 0;
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs

```rust
Lines: 787-791
        if full {
            // Start with wallpaper (full-opacity copy).
            dst.copy_from_slice(&self.layers[Layer::Wallpaper as usize].buf);

            // Blend each subsequent layer over the accumulated output.
```

```rust
Lines: 800-804
                        dst[i] = px;
                    } else {
                        dst[i] = alpha_blend(dst[i], px, (px >> 24) as u8);
                    }
                }
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\sys\lifecycle.rs

```rust
Lines: 26-30

/// Classifies a raw wait4 status into how the process exited.
pub(crate) fn exit_class(status: i32) -> ExitClass {
    if status == 0 {
        ExitClass::Clean
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\sec\a11y\focus.rs

```rust
Lines: 37-41
    }

    pub fn move_focus(&mut self, dir: FocusDirection, tree: &A11yTree) -> bool {
        match dir {
            FocusDirection::Next => {
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\io.rs

```rust
Lines: 561-565

/// Retrieves the current mouse state for a window.
pub fn get_mouse(handle: u64) -> MouseState {
    // SAFETY: gui get_mouse syscall is safe here
    let packed = unsafe { crate::syscall::syscall1(120, handle) };
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\ipc\transport.rs

```rust
Lines: 58-62
            .map(|c| PollFd { fd: c.fd, events: POLLIN, revents: 0 })
            .collect();
        let ready = match libsarga::net::poll(&mut pollfds, 0) {
            Ok(n) => n,
            Err(_) => return out,
```
