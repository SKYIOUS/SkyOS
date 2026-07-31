# SkyOS Boot to Desktop: Userspace Execution Flow

**Codemap ID:** SkyOS_Boot_to_Desktop__Userspace_Execution_Flow_20260731_151322

**Description:** Maps the complete userspace boot sequence from init (PID 1) through login to the ADE desktop environment, including process launch, IPC transport, GUI rendering pipeline, and syscall boundaries. Key entry points: init boot [1a], login authentication [2b], ADE event loop [3d], app spawning [4c], rendering pipeline [5b], IPC message flow [6c].

---

## Trace 1: System Boot: Init Process Startup

**Description:** PID 1 init process mounts essential filesystems and spawns system services (login-manager, svc)

### Trace Diagram

```
Init Process (PID 1) Boot Sequence
├── user_main() entry <-- 1a
│   ├── Mount essential filesystems
│   │   ├── io::mount("/tmp", "tmpfs") <-- 1b
│   │   ├── io::mount("/dev", "devfs") <-- main.rs:65
│   │   └── io::mount("/ctl", "ctlfs") <-- main.rs:68
│   ├── Build service list
│   │   ├── Service { name: "login-manager" } <-- 1c
│   │   └── Service { name: "svc" } <-- main.rs:80
│   ├── Spawn all services <-- 1d
│   │   └── svc.spawn() <-- main.rs:23
│   │       ├── process::fork() <-- 1e
│   │       └── Child: process::execve() <-- 1f
│   └── Main supervision loop <-- main.rs:90
│       └── process::waitpid(-1) <-- 1g
│           └── Respawn if service exits <-- main.rs:106
└── sarga_main! macro <-- main.rs:128
    └── Calls user_main()
```

### Location Details

**Location ID 1a: Init entry point**
- **Description:** PID 1 init process starts, becomes parent of all userspace
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:54

**Location ID 1b: Mount tmpfs**
- **Description:** Mounts essential filesystems: /tmp, /dev, /ctl via syscalls
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:62

**Location ID 1c: Register login-manager service**
- **Description:** Adds login-manager to service list for spawning
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:73

**Location ID 1d: Spawn services**
- **Description:** Forks and execs each service (login-manager, svc)
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:87

**Location ID 1e: Fork child process**
- **Description:** Service::spawn() creates new process via fork syscall
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:28

**Location ID 1f: Exec service binary**
- **Description:** Replaces child process image with service executable
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:31

**Location ID 1g: Reap child processes**
- **Description:** Init's main loop waits for children, respawns services on exit
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:96

---

## Trace 2: User Login: Authentication to Desktop Launch

**Description:** Login-manager authenticates user credentials and launches ADE desktop via execve

### Trace Diagram

```
Login-Manager Process Flow
├── user_main() entry <-- 2a
│   ├── Window::create() for login UI <-- main.rs:21
│   ├── Event loop: keyboard input <-- main.rs:46
│   │   └── On Enter key pressed <-- main.rs:51
│   │       ├── verify_password() <-- 2b
│   │       │   └── Read /etc/shadow <-- 2c
│   │       │       └── PBKDF2 hash check <-- main.rs:16
│   │       └── if authenticated
│   │           └── process::execve() <-- 2d
│   │               └── libsarga::process::execve() <-- 2e
│   │                   └── syscall3(SYS_EXECVE)
│   │                       └── [KERNEL: replaces process]
│   │                           └── /bin/ade starts
│   └── On failure: clear password, show error <-- main.rs:63
└── sarga_main! macro expands to main() <-- main.rs:233
```

### Location Details

**Location ID 2a: Login-manager entry**
- **Description:** Graphical login screen starts, creates window for authentication
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\login-manager\src\main.rs:19

**Location ID 2b: Verify credentials**
- **Description:** Checks username/password against /etc/shadow via PBKDF2
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\login-manager\src\main.rs:54

**Location ID 2c: Password verification**
- **Description:** Reads shadow file and validates hash
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\login-manager\src\main.rs:11

**Location ID 2d: Launch ADE desktop**
- **Description:** Replaces login-manager process with ADE desktop environment
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\login-manager\src\main.rs:55

**Location ID 2e: Execve syscall**
- **Description:** Invokes SYS_EXECVE (59) to replace process image with /bin/ade
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\process.rs:266

---

## Trace 3: ADE Desktop Initialization & Event Loop

**Description:** ADE desktop environment creates window, initializes compositor and desktop state, enters main event loop

### Trace Diagram

```
ADE Desktop Initialization & Event Loop
├── user_main() entry <-- 3a
│   ├── Window::create() <-- 3b
│   │   └── syscall3(SYS_GUI_CREATE_WINDOW) <-- gui.rs:424
│   │       └── kernel allocates framebuffer
│   ├── Desktop::new() <-- 3c
│   │   ├── WindowManager::new() <-- desktop.rs:149
│   │   ├── ServiceManager::new() <-- desktop.rs:180
│   │   ├── IpcTransport::new() <-- desktop.rs:202
│   │   └── Compositor::new() <-- main.rs:33
│   └── while running { ... } <-- 3d
│       ├── desktop.tick() <-- 3e
│       │   ├── advance_clock() <-- desktop.rs:257
│       │   ├── reap_children() <-- desktop.rs:272
│       │   ├── ipc_transport.ingest() <-- desktop.rs:273
│       │   └── services.tick() <-- desktop.rs:301
│       ├── poll keyboard/mouse input <-- main.rs:48
│       ├── desktop.handle_event() <-- main.rs:55
│       ├── render::render() <-- 3f
│       │   └── compositor.compose() <-- mod.rs:208
│       ├── desktop_win.flush() <-- 3g
│       │   └── syscall2(SYS_GUI_FLUSH) <-- gui.rs:475
│       └── nanosleep(16ms) <-- main.rs:97
```

### Location Details

**Location ID 3a: ADE entry point**
- **Description:** Desktop environment starts after successful login
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:21

**Location ID 3b: Create desktop window**
- **Description:** Allocates kernel-backed framebuffer via SYS_GUI_CREATE_WINDOW
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:24

**Location ID 3c: Initialize desktop state**
- **Description:** Creates window manager, services, IPC transport, compositor
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:32

**Location ID 3d: Main event loop**
- **Description:** 60 FPS loop processing input, ticking services, rendering frames
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:45

**Location ID 3e: Tick desktop state**
- **Description:** Advances clock, reaps children, processes IPC, animates windows
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:46

**Location ID 3f: Render frame**
- **Description:** Composites all layers (wallpaper, windows, popups, cursor) to framebuffer
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:81

**Location ID 3g: Flush to display**
- **Description:** Sends framebuffer to kernel compositor via SYS_GUI_FLUSH
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:82

---

## Trace 4: Application Launch: Fork, Exec, IPC Registration

**Description:** Desktop spawns new application via fork/execve, registers IPC socketpair, creates window in window manager

### Trace Diagram

```
Application Launch Flow (Trace 4)
├── User clicks app icon in Start Menu
│   └── Desktop::launch_app() <-- 4a
│       └── lookup app in registry <-- desktop.rs:845
│           └── spawn_app_from_registry() <-- 4b
│               ├── Create socketpair for IPC <-- launcher.rs:94
│               ├── process::fork() <-- 4c
│               │   ├── Child process (pid=0) <-- launcher.rs:101
│               │   │   └── process::execve() <-- 4d
│               │   │       └── [kernel replaces image]
│               │   └── Parent process (ADE) <-- launcher.rs:115
│               │       ├── ipc_transport.register() <-- 4e
│               │       ├── lifecycle.register() <-- 4f
│               │       ├── permissions.register() <-- launcher.rs:123
│               │       └── wm.create(app_win) <-- 4g
│               │           └── [window added to list] <-- window_manager.rs:46
│               └── animate window fade-in <-- launcher.rs:145
└── App now running with IPC channel
```

### Location Details

**Location ID 4a: Launch app from registry**
- **Description:** User clicks app icon, desktop looks up executable path
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:843

**Location ID 4b: Delegate to launcher**
- **Description:** Calls launcher module to fork/exec the application
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:875

**Location ID 4c: Fork child process**
- **Description:** Creates new process for application via SYS_FORK syscall
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:100

**Location ID 4d: Exec application binary**
- **Description:** Replaces child process image with app executable
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:107

**Location ID 4e: Register IPC channel**
- **Description:** Maps PID to socketpair FD for IPC communication
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:127

**Location ID 4f: Track process lifecycle**
- **Description:** Registers PID in lifecycle manager for crash detection
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:122

**Location ID 4g: Create window**
- **Description:** Adds window to window manager's ordered list
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\launcher.rs:142

---

## Trace 5: GUI Rendering Pipeline: Compositor to Kernel

**Description:** Multi-layer compositor renders desktop (wallpaper, windows, popups, cursor) and flushes to kernel framebuffer

### Trace Diagram

```
ADE Rendering Pipeline (Trace 5)
│
├── Event Loop (ade/src/main.rs)
│   └── render::render() call <-- 5a
│
├── Render Function (ade/src/render/mod.rs)
│   ├── comp.clear_all() <-- 5b
│   │   └── Reset 6 layer buffers <-- compositor.rs:694
│   │
│   ├── Layer Drawing Phase
│   │   ├── Wallpaper layer <-- mod.rs:24
│   │   │   └── crate::core::wallpaper::draw() <-- mod.rs:25
│   │   ├── Desktop layer (icons) <-- mod.rs:41
│   │   ├── Windows layer <-- mod.rs:47
│   │   │   └── for aw in snap.windows <-- 5c
│   │   │       └── crate::core::window::draw() <-- mod.rs:53
│   │   ├── Popups layer <-- mod.rs:67
│   │   │   └── taskbar::draw() <-- 5d
│   │   ├── Overlay layer (menus, notifications) <-- mod.rs:77
│   │   └── Cursor layer <-- mod.rs:172
│   │
│   └── comp.compose() <-- 5e
│
└── Compositor (ade/src/render/compositor.rs)
    └── compose() method <-- compositor.rs:781
        ├── Copy wallpaper base <-- 5f
        └── Blend layers loop <-- compositor.rs:792
            └── alpha_blend() <-- 5g
                └── Output to window framebuffer <-- compositor.rs:800
```

### Location Details

**Location ID 5a: Render entry point**
- **Description:** Main rendering function called each frame from event loop
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs:14

**Location ID 5b: Clear compositor layers**
- **Description:** Resets all 6 offscreen buffers (wallpaper, desktop, windows, popups, overlay, cursor)
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs:20

**Location ID 5c: Render windows layer**
- **Description:** Draws each window with shadows, decorations, content to Windows layer
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs:47

**Location ID 5d: Render taskbar**
- **Description:** Draws taskbar with start button, window tabs, clock to Popups layer
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs:68

**Location ID 5e: Composite layers**
- **Description:** Alpha-blends all layers onto window framebuffer in order
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs:208

**Location ID 5f: Copy wallpaper base**
- **Description:** Starts composition with opaque wallpaper layer
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs:789

**Location ID 5g: Alpha blend layers**
- **Description:** Blends each subsequent layer pixel-by-pixel onto output
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\compositor.rs:802

---

## Trace 6: IPC Message Flow: Transport to Service Dispatch

**Description:** ADE processes IPC requests from apps via socketpair transport, dispatches to services, returns responses

### Trace Diagram

```
ADE Desktop IPC Message Flow
├── Desktop::tick() event loop <-- desktop.rs:255
│   ├── ipc_transport.ingest() <-- 6a
│   │   ├── poll(socketpairs, timeout=0) <-- 6b
│   │   ├── for each ready FD <-- transport.rs:68
│   │   │   ├── read_frame(fd, buf) <-- 6c
│   │   │   └── decode_request(buf) <-- 6d
│   │   └── return Vec<ServiceRequest> <-- transport.rs:101
│   ├── ipc_server.submit_request(req) <-- 6e
│   │   └── queue request for dispatch
│   ├── process_ipc() (service dispatch) <-- desktop.rs:277
│   ├── ipc_server.drain_responses() <-- 6f
│   └── ipc_transport.deliver(responses) <-- desktop.rs:279
│       ├── for each response <-- transport.rs:108
│       │   ├── encode_response(req_id, data) <-- 6g
│       │   └── write_frame(fd, frame) <-- 6h
│       └── unregister dead connections <-- transport.rs:121
└── 60 FPS loop continues
```

### Location Details

**Location ID 6a: Ingest IPC requests**
- **Description:** Desktop tick polls all app socketpairs for incoming messages
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:273

**Location ID 6b: Poll socketpairs**
- **Description:** Non-blocking poll checks which app FDs have data ready
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\ipc\transport.rs:60

**Location ID 6c: Read IPC frame**
- **Description:** Reads length-prefixed message from ready socketpair
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\ipc\transport.rs:76

**Location ID 6d: Decode request**
- **Description:** Parses wire format: request_id | service | method | args
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\ipc\transport.rs:79

**Location ID 6e: Submit to IPC server**
- **Description:** Queues request for service dispatch
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:275

**Location ID 6f: Drain responses**
- **Description:** Collects all service responses ready for delivery
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:278

**Location ID 6g: Encode response**
- **Description:** Serializes response: request_id | success | data
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\ipc\transport.rs:111

**Location ID 6h: Write response frame**
- **Description:** Sends response back to app via socketpair
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\ipc\transport.rs:116

---

## Trace 7: Syscall Boundary: Userspace to Kernel

**Description:** Userspace GUI operations cross into kernel via inline assembly syscall instruction

### Trace Diagram

```
Userspace GUI Window Creation → Kernel Syscall
└── Window::create() API <-- 7a
    └── unsafe block preparation <-- 7b
        └── syscall3() invocation <-- 7c
            ├── SYS_GUI_CREATE_WINDOW (100) <-- 7d
            └── syscall3() wrapper function <-- 7e
                └── inline asm block <-- 7f
                    └── "syscall" instruction <-- 7g
                        └── [KERNEL MODE TRANSITION]
                            └── (kernel syscall handler)
                                └── (missing: kernel code
                                    not in this repo)
```

### Location Details

**Location ID 7a: Window::create API**
- **Description:** Userspace window creation entry point
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\gui.rs:421

**Location ID 7b: Prepare syscall**
- **Description:** Marshals arguments for kernel syscall
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\gui.rs:423

**Location ID 7c: Invoke syscall3**
- **Description:** Calls syscall wrapper with 3 arguments
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\gui.rs:424

**Location ID 7d: GUI syscall number**
- **Description:** Syscall 100: creates window in kernel compositor
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\gui.rs:425

**Location ID 7e: Syscall3 wrapper**
- **Description:** Generic 3-argument syscall function
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\syscall.rs:31

**Location ID 7f: Inline assembly**
- **Description:** x86_64 SYSCALL instruction: transitions to kernel mode
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\syscall.rs:7

**Location ID 7g: SYSCALL instruction**
- **Description:** CPU switches to ring 0, jumps to kernel syscall handler (MISSING: kernel code not in this repo)
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\syscall.rs:8

---

## Trace 8: Desktop Event Processing: Input to Damage Tracking

**Description:** Desktop event loop processes mouse/keyboard input, updates state, marks damaged regions for redraw

### Trace Diagram

```
ADE Desktop Event Loop (main.rs) <-- main.rs:45
├── while running { <-- 8a
│   ├── Keyboard Input Path
│   │   ├── desktop_win.get_key() <-- 8a
│   │   │   └── syscall1(SYS_GUI_GET_KEY) <-- gui.rs:466
│   │   └── desktop.handle_event(Key) <-- 8b
│   │       └── handle_event() dispatcher <-- 8e
│   │           └── handle_click() / handle_key()
│   │               └── handle_click(x, y) <-- 8f
│   ├── Mouse Input Path
│   │   ├── desktop_win.get_mouse() <-- 8c
│   │   │   └── kernel mouse state read <-- gui.rs:869
│   │   └── desktop.handle_event(MouseClick) <-- 8d
│   │       └── [routes to handle_event above]
│   └── Rendering Decision
│       ├── if desktop.damage.is_dirty() <-- 8g
│       │   ├── desktop.prepare_clock() <-- main.rs:79
│       │   ├── render(&mut desktop_win, ...) <-- main.rs:81
│       │   └── desktop_win.flush() <-- main.rs:82
│       └── desktop.damage.clear() <-- 8h
└── Frame sleep (16ms target) <-- main.rs:98
```

### Location Details

**Location ID 8a: Poll keyboard input**
- **Description:** Reads keyboard events from kernel via SYS_GUI_GET_KEY
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:48

**Location ID 8b: Dispatch key event**
- **Description:** Routes keyboard input to desktop event handler
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:55

**Location ID 8c: Poll mouse state**
- **Description:** Reads mouse position and buttons from kernel
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:59

**Location ID 8d: Dispatch mouse click**
- **Description:** Routes mouse click to desktop event handler
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:66

**Location ID 8e: Handle event**
- **Description:** Central event dispatcher processes input events
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:564

**Location ID 8f: Process click**
- **Description:** Hit-tests windows, taskbar, icons; updates focus
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs:573

**Location ID 8g: Check damage**
- **Description:** Determines if any screen region needs redraw
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:78

**Location ID 8h: Clear damage**
- **Description:** Resets damage tracker after frame rendered
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:85

---

## Code Snippets

### File: c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs

```rust
Lines: 26-33
        let _ = io::write_all(1, b"\n");

        match process::fork() {
            Ok(0) => {
                // Child
                if let Err(_e) = process::execve(&self.exec, &[], &[]) {
                    let _ = io::write_all(1, b"[init] exec failed for ");
                    let _ = io::write_all(1, self.name.as_bytes());
```

```rust
Lines: 52-56
}

fn user_main() -> i32 {
    let _ = io::write_all(1, b"[init] SARGA init starting\n");
    let _ = io::write_all(1, b"Userland init running\n");
```

```rust
Lines: 60-64
    let _ = io::mkdir("/dev", 0o755);
    let _ = io::mkdir("/ctl", 0o755);
    if let Err(_) = io::mount("none", "/tmp", "tmpfs", 0) {
        let _ = io::write_all(1, b"[init] WARN: failed to mount /tmp\n");
    }
```

```rust
Lines: 71-75

    let mut services = Vec::new();
    services.push(Service {
        name: "login-manager".to_string(),
        exec: "/bin/login-manager".to_string(),
```

```rust
Lines: 85-89

    for svc in &mut services {
        let _ = svc.spawn();
    }
```

```rust
Lines: 94-98

        // Wait for any child process to exit (-1 means any child)
        match process::waitpid(-1, 0) {
            Ok((pid, _status)) => {
                let mut found = false;
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\process.rs

```rust
Lines: 264-268
    envp.push(core::ptr::null());

    let r = unsafe {
        syscall3(
            59,
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\core\desktop.rs

```rust
Lines: 271-280
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
```

```rust
Lines: 562-566
    }

    pub fn handle_event(&mut self, ev: Event) {
        match ev {
            Event::Key(key) => {
```

```rust
Lines: 571-575
            Event::MouseClick(x, y) => {
                self.focus_visible = false;
                self.handle_click(x, y);
            }
            Event::MouseMiddle(x, y) => {
```

```rust
Lines: 841-845
    }

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
Lines: 105-109
                        let fd_arg = alloc::format!("{}", client_fd);
                        let argv = [path, "--ipc-fd", fd_arg.as_str()];
                        let _ = libsarga::process::execve(path, &argv, &[]);
                    }
                    None => {
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

### File: c:\Users\nanda\Desktop\Github\SkyOS\login-manager\src\main.rs

```rust
Lines: 9-13
const SHADOW_PATH: &str = "/etc/shadow";

fn verify_password(username: &str, password: &str) -> bool {
    let data = match libsarga::fs::read_to_string(SHADOW_PATH) {
        Ok(d) => d.into_bytes(),
```

```rust
Lines: 17-21
}

fn user_main() -> i32 {
    let theme = Theme::dark();
    let mut win = match Window::create("SARGA OS", 800, 600) {
```

```rust
Lines: 52-57
                    let user = core::str::from_utf8(&username_buf).unwrap_or(...
                    let pass = core::str::from_utf8(&password_buf).unwrap_or(...
                    if verify_password(user, pass) {
                        match process::execve("/bin/ade", &["/bin/ade"], &[]) {
                            Ok(_) => return 0,
                            Err(_) => {
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs

```rust
Lines: 19-26
use render::compositor::Compositor;

fn user_main() -> i32 {
    io::print_str("[ade] starting desktop environment\n");

    let mut desktop_win = match Window::create("SARGA OS Desktop", 800, 600) {
        Ok(w) => w,
        Err(e) => {
```

```rust
Lines: 30-34
    };

    let mut desktop = Desktop::new(desktop_win.width, desktop_win.height);
    let mut compositor = Compositor::new(desktop_win.width, desktop_win.heigh...
    if (0..libsarga::args::argc()).any(|i| libsarga::args::get(i as usize) ==...
        let ok = util::testing::run_all(&mut desktop);
```

```rust
Lines: 43-50
    let mut running = true;
    let mut last_frame_ticks = 0u64;
    while running {
        desktop.tick();

        while let Some(key) = desktop_win.get_key() {
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
Lines: 76-87
        }

        if desktop.damage.is_dirty() {
            let clock_str = desktop.prepare_clock();
            let snap = desktop.snapshot();
            render::render(&mut desktop_win, &snap, &clock_str, &mut composit...
            if let Err(e) = desktop_win.flush() {
                io::print_str(&alloc::format!("[ade] flush error: {}\n", e));
            }
            desktop.damage.clear();
        }
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\render\mod.rs

```rust
Lines: 12-16
use compositor::Compositor;

pub(crate) fn render(
    win: &mut libsarga::gui::Window,
    snap: &snapshot::RenderSnapshot,
```

```rust
Lines: 18-22
    comp: &mut Compositor,
) {
    comp.clear_all();

    // Wallpaper
```

```rust
Lines: 45-49
    // Windows (normal then always-on-top)
    {
        let mut cv = comp.layer_canvas(Layer::Windows);
        for aw in snap.windows {
            if !aw.always_on_top {
```

```rust
Lines: 66-70
    if !snap.fullscreen {
        let mut cv = comp.layer_canvas(Layer::Popups);
        crate::core::taskbar::draw(&mut cv, snap, clock_str);

        if snap.start_menu {
```

```rust
Lines: 206-210
    }

    comp.compose(win, None);
}
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

### File: c:\Users\nanda\Desktop\Github\SkyOS\ade\src\ipc\transport.rs

```rust
Lines: 58-62
            .map(|c| PollFd { fd: c.fd, events: POLLIN, revents: 0 })
            .collect();
        let ready = match libsarga::net::poll(&mut pollfds, 0) {
            Ok(n) => n,
            Err(_) => return out,
```

```rust
Lines: 74-81
            }
            let mut buf = [0u8; MAX_IPC_MSG];
            match read_frame(self.peers[i].fd, &mut buf) {
                Ok(0) => dead.push(self.peers[i].pid),
                Ok(n) => {
                    match libsarga::ipc::decode_request(&buf[..n]) {
                        Some((req_id, service, method, args)) => {
                            match (ServiceId::from_wire(service), alloc::stri...
```

```rust
Lines: 109-118
            let pid = resp.recipient.0;
            if let Some(fd) = self.fd_for(pid) {
                let frame = libsarga::ipc::encode_response(
                    resp.request_id.0,
                    resp.success,
                    &resp.data,
                );
                if write_frame(fd, &frame).is_err() {
                    dead.push(pid);
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\gui.rs

```rust
Lines: 419-427

impl Window {
    pub fn create(title: &str, width: u32, height: u32) -> Result<Self, i64> {
        let title_c = format!("{}\0", title);
        let id = unsafe {
            syscall3(
                SYS_GUI_CREATE_WINDOW,
                title_c.as_ptr() as u64,
                width as u64,
```

### File: c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\syscall.rs

```rust
Lines: 5-10
pub unsafe fn syscall6(n: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a...
    let ret: i64;
    core::arch::asm!(
        "syscall",
        inout("rax") n => ret,
        in("rdi") a1, in("rsi") a2, in("rdx") a3,
```

```rust
Lines: 29-33
}
#[inline(always)]
pub unsafe fn syscall3(n: u64, a1: u64, a2: u64, a3: u64) -> i64 {
    syscall6(n, a1, a2, a3, 0, 0, 0)
}
```
