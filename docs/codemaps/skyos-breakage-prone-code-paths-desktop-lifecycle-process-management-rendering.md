# SkyOS Breakage-Prone Code Paths: Desktop Lifecycle, Process Management & Rendering

**Codemap ID:** SkyOS_Breakage_Prone_Code_Paths__Desktop_Lifecycle__Process_Management___Rendering_20260731_154945

**Description:** Identifies fragile code paths in the SkyOS desktop environment that are prone to breakage, including desktop lifecycle management, process spawning/reaping, window manager state handling, IPC permission checks, and compositor rendering. Key risks: unwrap() calls [3d], unbounded loops [6a], stale indices [7e], and OOM conditions [5b].

---

## Trace 1: Desktop Initialization & Compositor Allocation

**Description:** Desktop startup sequence with large memory allocation for compositor layers - can fail on low memory or large screens

### Trace Diagram

```
ADE Desktop Initialization
├── user_main() entry <-- main.rs:21
│   ├── Window::create() <-- main.rs:24
│   │   └── syscall3(SYS_GUI_CREATE_WINDOW) <-- gui.rs:424
│   │       └── kernel allocates framebuffer
│   ├── Desktop::new() <-- main.rs:32
│   │   ├── WindowManager::new() <-- desktop.rs:149
│   │   ├── ServiceManager::new() <-- desktop.rs:180
│   │   ├── IpcTransport::new() <-- desktop.rs:202
│   │   └── Compositor::new() <-- main.rs:33
│   │       ├── LayerBuffer::new(pixels) x6 <-- 1a
│   │       └── vec![0u32; w*h] per layer <-- 1b
│   └── Main event loop <-- main.rs:45
└── RISK: OOM on large screens (18MB at 1024x768, 75MB at 1920x1080)
```

### Location Details

**Location ID 1a: Layer buffer allocation**
- **Description:** Allocates 6 LayerBuffers, each width*height*4 bytes - can OOM on large screens
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs:677

**Location ID 1b: Per-layer pixel array**
- **Description:** 6 LayerBuffers, each width*height*4 bytes - can OOM on large screens
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs:652

**Breakage Risk:** Out-of-memory on high-resolution displays or low-memory systems

---

## Trace 2: Desktop Tick & Damage Tracking

**Description:** Per-frame desktop state updates with damage tracking - full-frame recomposite on any state change

### Trace Diagram

```
Desktop Frame Tick (every ~16ms)
├── desktop.tick() <-- desktop.rs:254
│   ├── advance_clock() <-- desktop.rs:257
│   ├── reap_children() <-- desktop.rs:272
│   ├── ipc_transport.ingest() <-- desktop.rs:273
│   ├── process_ipc() <-- desktop.rs:277
│   ├── services.tick() <-- desktop.rs:301
│   ├── wm.process_closing() <-- desktop.rs:296
│   └── build_a11y_tree() <-- desktop.rs:303
└── Damage Tracking
    ├── damage.mark_full() on any state change <-- 2a
    └── if damage.is_dirty() <-- main.rs:78
        └── Full-frame recomposite <-- 2b
```

### Location Details

**Location ID 2a: Mark full damage**
- **Description:** Sets dirty flag to trigger full-frame recomposite
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1166

**Location ID 2b: Full-frame recomposite**
- **Description:** Compositor recomposites entire framebuffer even for small changes
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs:789

**Breakage Risk:** Performance degradation - full-frame recomposite on every state change

---

## Trace 3: Application Launch & Process Spawning

**Description:** Desktop spawns applications via fork/execve with unwrap() calls that can panic

### Trace Diagram

```
Application Launch Flow
├── Desktop::spawn_app() <-- desktop.rs:1067
│   └── launcher::spawn_app() <-- launcher.rs:50
│       ├── Create socketpair for IPC <-- launcher.rs:94
│       ├── process::fork() <-- 3a
│       │   ├── Child branch (pid=0) <-- launcher.rs:101
│       │   │   └── process::execve() <-- launcher.rs:107
│       │   └── Parent branch (pid>0) <-- launcher.rs:115
│       │       ├── lifecycle.register(pid) <-- 3b
│       │       ├── permissions.register(pid) <-- launcher.rs:123
│       │       ├── ipc_transport.register(pid, server_fd) <-- 3c
│       │       └── wm.create(app_win) <-- 3d
│       └── animate window fade-in <-- launcher.rs:145
└── RISK: unwrap() on explorer_id can panic
```

### Location Details

**Location ID 3a: Process fork**
- **Description:** Creates child process - can fail if resource limits hit
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:100

**Location ID 3b: Lifecycle registration**
- **Description:** Tracks new process in lifecycle manager
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:122

**Location ID 3c: IPC transport registration**
- **Description:** Associates socketpair FD with process for IPC
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:127

**Location ID 3d: Window manager registration**
- **Description:** Adds window to WM list, returns WindowId
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:142

**Location ID 3e: Unwrap on explorer ID**
- **Description:** PANIC RISK: Assumes explorer_id is Some without checking
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1433

**Breakage Risk:** Panic on unwrap() calls, fork failures, resource exhaustion

---

## Trace 4: IPC Request Processing & Permission Gating

**Description:** IPC subsystem - ingests requests from transport, checks permissions, dispatches to portal. Bounded to 64 requests/frame.

### Trace Diagram

```
IPC Request Processing Pipeline
├── Desktop tick() - main event loop <-- desktop.rs:255
│   ├── ipc_transport.ingest() <-- 4a
│   │   ├── poll(pollfds, timeout=0) <-- 4b
│   │   ├── read_frame() from ready FDs <-- transport.rs:76
│   │   └── decode_request() & queue <-- transport.rs:79
│   ├── ipc_server.submit_request() <-- desktop.rs:275
│   └── process_ipc() <-- 4c
│       ├── drain_requests() <-- desktop.rs:1749
│       ├── throttle check (MAX=64) <-- 4d
│       ├── for each request: <-- desktop.rs:1753
│       │   ├── permissions.granted(pid) <-- desktop.rs:1755
│       │   ├── service_registry.find() <-- desktop.rs:1758
│       │   ├── permission gate check <-- 4e
│       │   └── portal::dispatch() if allowed <-- desktop.rs:1766
│       └── submit_response() <-- 4f
└── ipc_transport.deliver() <-- desktop.rs:279
    └── write_frame() to peer FDs <-- transport.rs:116
```

### Location Details

**Location ID 4a: IPC transport ingestion**
- **Description:** Polls all peer FDs, reads available request frames
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:273

**Location ID 4b: Non-blocking poll**
- **Description:** Checks which peer FDs have data ready without blocking
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\ipc\transport.rs:60

**Location ID 4c: IPC processing entry**
- **Description:** Permission-gates and dispatches queued requests
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:277

**Location ID 4d: Request throttling**
- **Description:** Caps processing at 64 requests/frame to prevent stalls
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1750

**Location ID 4e: Permission check**
- **Description:** Verifies caller has required permissions for service
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1756

**Location ID 4f: Response queuing**
- **Description:** Queues response for delivery back to caller
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:1775

**Breakage Risk:** Dual permission systems, request throttling may drop messages

---

## Trace 5: Compositor Layer Allocation & Frame Composition

**Description:** Rendering subsystem - manages 6 screen-sized layer buffers, composites to window. Memory-intensive, allocates on startup.

### Trace Diagram

```
ADE Main Entry & Compositor Initialization
├── user_main() entry <-- main.rs:21
│   ├── Window::create() <-- 5a
│   │   └── Compositor::new(w, h) <-- compositor.rs:674
│   │       ├── LayerBuffer::new(pixels) x6 <-- 5b
│   │       └── vec![0u32; w*h] per layer <-- compositor.rs:652
│   └── Main event loop <-- main.rs:45
│       └── if damage.is_dirty() <-- main.rs:78
│           └── render::render() <-- main.rs:81
│               ├── comp.clear_all() <-- 5c
│               │   └── Zeros all 6 layer buffers <-- compositor.rs:694
│               ├── Draw to layer canvases
│               │   ├── layer_canvas(Wallpaper) <-- mod.rs:24
│               │   ├── layer_canvas(Windows) <-- mod.rs:47
│               │   └── layer_canvas(Cursor) <-- mod.rs:173
│               └── comp.compose(win, None) <-- 5d
│                   ├── Wallpaper base copy <-- 5e
│                   └── Blend layers 1-5 <-- compositor.rs:792
│                       └── alpha_blend() <-- 5f
```

### Location Details

**Location ID 5a: Compositor initialization**
- **Description:** Allocates 6 layer buffers at screen resolution - large memory allocation
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:33

**Location ID 5b: Layer buffer array**
- **Description:** 6 LayerBuffers, each width*height*4 bytes - can OOM on large screens
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs:677

**Location ID 5c: Layer clearing**
- **Description:** Zeros all layer buffers each frame - O(width*height*6)
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs:20

**Location ID 5d: Layer composition**
- **Description:** Alpha-blends all 6 layers into window buffer - CPU intensive
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs:208

**Location ID 5e: Wallpaper copy**
- **Description:** Full-buffer memcpy of wallpaper layer as base
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs:789

**Location ID 5f: Per-pixel alpha blend**
- **Description:** Blends each non-transparent pixel - hot loop
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs:802

**Breakage Risk:** OOM on startup, performance degradation on high-resolution displays

---

## Trace 6: Init Service Respawn Loop

**Description:** Init system - separate from ADE, manages service lifecycle with unbounded respawn. Potential infinite loop if service crashes immediately.

### Trace Diagram

```
Init Process (PID 1)
├── user_main() entry <-- main.rs:54
│   ├── Mount filesystems (/tmp, /dev, /ctl) <-- main.rs:59
│   ├── Create service list <-- main.rs:72
│   │   ├── login-manager service <-- main.rs:73
│   │   └── svc service <-- main.rs:79
│   ├── Initial spawn of all services <-- main.rs:86
│   └── Main supervision loop <-- 6a
│       └── while !SHUTDOWN <-- main.rs:91
│           ├── waitpid(-1, 0) blocks <-- 6b
│           ├── Service exit detected <-- main.rs:97
│           │   └── Find service by PID <-- main.rs:99
│           │       ├── Check respawn flag <-- 6c
│           │       ├── nanosleep(500ms) <-- 6d
│           │       └── svc.spawn() <-- 6e
│           │           └── process::fork() <-- 6f
│           │               ├── Child: execve(service) <-- main.rs:31
│           │               └── Parent: store PID <-- main.rs:41
│           └── Handle orphaned processes <-- main.rs:114
└── Shutdown cleanup <-- main.rs:124
```

### Location Details

**Location ID 6a: Init main loop**
- **Description:** Unbounded loop waiting for child exits
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:90

**Location ID 6b: Blocking waitpid**
- **Description:** Waits for any child to exit - blocks until signal
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:96

**Location ID 6c: Respawn check**
- **Description:** Determines if service should be restarted
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:106

**Location ID 6d: Respawn delay**
- **Description:** 500ms delay before respawn - prevents tight crash loop
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:107

**Location ID 6e: Service respawn**
- **Description:** Forks and execs service again - no crash count limit
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:108

**Location ID 6f: Service fork**
- **Description:** Creates child process for service
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:28

**Breakage Risk:** Infinite respawn loop if service crashes immediately, no crash count limit

---

## Trace 7: Window Manager State & Close Animation

**Description:** Window manager subsystem - tracks window list, handles close with animation. Stale indices can cause out-of-bounds access.

### Trace Diagram

```
Window Manager Close Flow
├── Public API
│   └── wm.close(id) <-- 7a
│       └── Mark window closing flag <-- 7b
│           └── w.animate_close() <-- window_manager.rs:57
├── Tick Processing (each frame)
│   └── desktop.tick() <-- main.rs:46
│       └── wm.process_closing() <-- 7c
│           ├── Iterate windows backwards <-- window_manager.rs:74
│           │   └── Check animation done <-- 7d
│           │       └── windows.remove(i) <-- 7e
│           └── Update focused index <-- window_manager.rs:82
└── Alternative: Close by PID
    └── wm.close_by_pid(pid) <-- 7f
        └── Find window by PID <-- window_manager.rs:63
            └── windows.remove(pos) <-- window_manager.rs:64
```

### Location Details

**Location ID 7a: Window close entry**
- **Description:** Marks window for closing, starts animation
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\window_manager.rs:53

**Location ID 7b: Closing flag set**
- **Description:** Marks window as closing but doesn't remove yet
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\window_manager.rs:56

**Location ID 7c: Process closing windows**
- **Description:** Removes windows whose close animation finished
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:296

**Location ID 7d: Animation completion check**
- **Description:** Only removes window after animation completes
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\window_manager.rs:77

**Location ID 7e: Window removal**
- **Description:** Removes from Vec, shifts all subsequent indices down
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\window_manager.rs:79

**Location ID 7f: Close by PID lookup**
- **Description:** Finds window by process ID - can fail if multiple windows per process
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\window_manager.rs:63

**Breakage Risk:** Stale indices after Vec::remove, out-of-bounds access, animation state desync

---

## Code Snippets

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs

```rust
Lines: 31-35

    let mut desktop = Desktop::new(desktop_win.width, desktop_win.height);
    let mut compositor = Compositor::new(desktop_win.width, desktop_win.heigh...
    if (0..libsarga::args::argc()).any(|i| libsarga::args::get(i as usize) ==...
        let ok = util::testing::run_all(&mut desktop);
```

```rust
Lines: 43-48
    let mut running = true;
    let mut last_frame_ticks = 0u64;
    while running {
        desktop.tick();

        while let Some(key) = desktop_win.get_key() {
```

```rust
Lines: 76-84
        }

        if desktop.damage.is_dirty() {
            let clock_str = desktop.prepare_clock();
            let snap = desktop.snapshot();
            render::render(&mut desktop_win, &snap, &clock_str, &mut composit...
            if let Err(e) = desktop_win.flush() {
                io::print_str(&alloc::format!("[ade] flush error: {}\n", e));
            }
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs

```rust
Lines: 98-102
        )
        .ok();
        match libsarga::process::fork() {
            Ok(0) => {
                match ipc_pair {
```

```rust
Lines: 120-129
                    .map(|id| id.0)
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

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs

```rust
Lines: 18-22
    comp: &mut Compositor,
) {
    comp.clear_all();

    // Wallpaper
```

```rust
Lines: 206-210
    }

    comp.compose(win, None);
}
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\window_manager.rs

```rust
Lines: 51-58

    /// WindowManager API v1.0
    pub fn close(&mut self, id: WindowId) {
        if let Some(w) = self.windows.get_mut(id.0) {
            if !w.closing {
                w.closing = true;
                w.animate_close();
            }
```

```rust
Lines: 61-65

    pub fn close_by_pid(&mut self, pid: u64) {
        if let Some(pos) = self.windows.iter().position(|w| w.pid == Some(pid...
            self.windows.remove(pos);
            if self.windows.is_empty() {
                self.focused = None;
```

```rust
Lines: 75-81
        while i > 0 {
            i -= 1;
            if self.windows[i].closing && self.windows[i].anim.is_none() {
                closed.push(WindowId(i));
                self.windows.remove(i);
            }
        }
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs

```rust
Lines: 224-235

    pub fn reap_children(&mut self) {
        loop {
            match process::waitpid(-1, 1) {
                Ok((pid, status)) if pid > 0 => {
                    use crate::sys::lifecycle::ExitClass;
                    match crate::sys::lifecycle::exit_class(status) {
                        ExitClass::Clean => self.lifecycle.mark_terminated(pi...
                        cls => {
                            self.lifecycle.mark_crashed(pid);
                            let reason = match cls {
                                ExitClass::Killed => alloc::string::String::f...
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
Lines: 254-258

    pub fn tick(&mut self) {
        self.profiler.frame_timer.start(self.clock_ticks);
        self.advance_clock();
        // Breathing cursor: smooth alpha blink over 30 ticks
```

```rust
Lines: 270-279
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
```

```rust
Lines: 294-298
        }
        // Process closing windows (remove after shrink animation)
        let closed = self.wm.process_closing();
        for cid in closed {
            self.wm.close(cid);
```

```rust
Lines: 1065-1069
    }

    pub(crate) fn spawn_app(&mut self, path: &str, title: &str) {
        crate::core::launcher::spawn_app(self, path, title);
    }
```

```rust
Lines: 1431-1435
                let is_explorer = { self.wm.iter()[i].explorer_id.is_some() };
                if is_explorer {
                    let exp_id = self.wm.iter()[i].explorer_id.unwrap();
                    if let Some(exp_state) = self.explorers.iter_mut().find(|...
                        let aw_ref = &self.wm.iter()[i];
```

```rust
Lines: 1748-1752
        const MAX_REQUESTS_PER_FRAME: usize = 64;
        let mut requests = self.ipc_server.drain_requests();
        if requests.len() > MAX_REQUESTS_PER_FRAME {
            self.ipc_server.pending_requests = requests.split_off(MAX_REQUEST...
```

```rust
Lines: 1754-1758
            let app = req.sender;
            let granted = self.permissions.granted(app.0);
            let allowed = self
                .service_registry
                .find(req.service)
```

```rust
Lines: 1773-1777
                }
            };
            self.ipc_server.submit_response(resp);
        }
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

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs

```rust
Lines: 675-679
        let pixels = (w * h) as usize;
        Compositor {
            layers: [
                LayerBuffer::new(pixels),
                LayerBuffer::new(pixels),
```

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

### File: c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs

```rust
Lines: 26-30
        let _ = io::write_all(1, b"\n");

        match process::fork() {
            Ok(0) => {
                // Child
```

```rust
Lines: 88-92
    }

    loop {
        if SHUTDOWN.load(Ordering::Acquire) {
            break;
```

```rust
Lines: 94-98

        // Wait for any child process to exit (-1 means any child)
        match process::waitpid(-1, 0) {
            Ok((pid, _status)) => {
                let mut found = false;
```

```rust
Lines: 104-110

                        svc.pid = None;
                        if svc.respawn {
                            let _ = io::nanosleep(500_000_000);
                            let _ = svc.spawn();
                        }
                        found = true;
```
