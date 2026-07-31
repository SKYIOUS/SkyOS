# SkyOS Boot-to-Userspace Path: UEFI → Kernel → PID 1 → Desktop Shell

**Codemap ID:** SkyOS_Boot-to-Userspace_Path__UEFI___Kernel___PID_1___Desktop_Shell_20260731_154717

**Description:** Complete boot sequence from firmware to running desktop environment, covering kernel boot state machine (documented), init process startup, service management, login flow, and userspace entry mechanism. Key transitions: kernel enters userspace at [2e], init spawns services at [3c], login launches desktop at [4d], userspace binaries start at [5a].

---

## Trace 1: UEFI Firmware → Bootloader → Kernel Entry

**Description:** Hardware boot sequence and kernel initialization (external kernel repo, documented flow)

### Trace Diagram

```
System Boot Sequence
├── Power On
│   └── UEFI Firmware Execution <-- 1a
│       └── Initialize hardware & POST
├── Boot Device Selection
│   └── Load Bootloader from ESP <-- 1b
│       └── UEFI loads EFI application
├── Bootloader Execution
│   └── Load Kernel ELF & Setup Memory <-- 1c
│       ├── Parse kernel ELF binary
│       ├── Setup higher-half memory map
│       ├── Map framebuffer for graphics
│       └── Prepare BootInfo structure
└── Kernel Entry
    └── Jump to kernel_main() <-- 1d
        ├── Receive BootInfo (memory map, FB)
        ├── Initialize memory management <-- ARCHITECTURE.md:24
        ├── Setup interrupts & scheduler <-- ARCHITECTURE.md:31
        └── Enter boot state machine <-- 2026-07-26-boot-reliability-phase1-design.md:47
            └── [continues to trace 2]
```

### Location Details

**Location ID 1a: UEFI Firmware Execution**
- **Description:** System powers on, UEFI firmware initializes hardware
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\docs\ARCHITECTURE.md:15

**Location ID 1b: Bootloader Loading**
- **Description:** UEFI loads bootloader from ESP partition
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\docs\ARCHITECTURE.md:16

**Location ID 1c: Kernel ELF Loading**
- **Description:** Bootloader loads kernel, sets up memory map and framebuffer
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\docs\ARCHITECTURE.md:17

**Location ID 1d: Kernel Entry Point**
- **Description:** Control transfers to kernel_main with boot information
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\docs\ARCHITECTURE.md:18

---

## Trace 2: Kernel Boot State Machine → Userspace Transition

**Description:** Kernel's planned boot state machine that locates init binary, loads ELF, creates address space, and enters userspace (documented design, implementation status unclear)

### Trace Diagram

```
Kernel Boot State Machine Flow
├── Boot Initialization
│   └── kernel_main() entry <-- ARCHITECTURE.md:18
│       └── init_os_task() spawned <-- 2026-07-26-boot-reliability-phase1.md:473
│           └── Boot state machine starts <-- 2026-07-26-boot-reliability-phase1.md:218
│               ├── LocateInit state <-- 2a
│               │   └── Search paths <-- 2026-07-26-boot-reliability-phase1.md:321
│               ├── ParseElf state <-- 2b
│               │   └── Validate ELF header <-- 2026-07-26-boot-reliability-phase1.md:342
│               ├── CreateAddressSpace state <-- 2c
│               │   └── Allocate page tables <-- 2026-07-26-boot-reliability-phase1.md:354
│               ├── MapStack state <-- 2026-07-26-boot-reliability-phase1.md:370
│               │   └── Setup user stack <-- 2026-07-26-boot-reliability-phase1.md:375
│               ├── CreatePid1 state <-- 2026-07-26-boot-reliability-phase1.md:382
│               │   └── Register process <-- 2026-07-26-boot-reliability-phase1.md:392
│               ├── SetupConsole state <-- 2d
│               │   └── Open /dev/tty0 <-- 2026-07-26-boot-reliability-phase1.md:405
│               └── EnterUserspace state <-- 2e
│                   ├── Activate address space <-- 2026-07-26-boot-reliability-phase1.md:444
│                   ├── jump_to_usermode() <-- 2f
│                   └── → PID 1 _start() <-- start.rs:3
```

### Location Details

**Location ID 2a: Locate Init Binary**
- **Description:** Boot state machine searches for init executable in standard paths
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\docs\superpowers\specs\2026-07-26-boot-reliability-phase1-design.md:48

**Location ID 2b: Parse Init ELF**
- **Description:** Validate ELF header and prepare for loading
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\docs\superpowers\specs\2026-07-26-boot-reliability-phase1-design.md:49

**Location ID 2c: Create Address Space**
- **Description:** Allocate new page tables for PID 1 process
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\docs\superpowers\specs\2026-07-26-boot-reliability-phase1-design.md:50

**Location ID 2d: Setup Console I/O**
- **Description:** Connect stdin/stdout/stderr to /dev/tty0 or serial
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\docs\superpowers\specs\2026-07-26-boot-reliability-phase1-design.md:53

**Location ID 2e: Enter Userspace**
- **Description:** Activate address space and jump to usermode with init entry point
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\docs\superpowers\specs\2026-07-26-boot-reliability-phase1-design.md:54

**Location ID 2f: Userspace Entry Mechanism**
- **Description:** Uses same path as spawn_userspace_app for consistent usermode transition
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\docs\superpowers\specs\2026-07-26-boot-reliability-phase1-design.md:138

---

## Trace 3: Init Process (PID 1) Startup & Service Spawning

**Description:** First userspace process mounts essential filesystems, spawns login-manager and svc services, then enters reap/respawn loop

### Trace Diagram

```
Init Process (PID 1) Execution Flow
├── user_main() entry <-- 3a
│   ├── Mount essential filesystems
│   │   ├── io::mount("/tmp", "tmpfs") <-- 3b
│   │   ├── io::mount("/dev", "devfs") <-- main.rs:65
│   │   └── io::mount("/ctl", "ctlfs") <-- main.rs:68
│   ├── Initialize service list
│   │   ├── Service { name: "login-manager" } <-- main.rs:73
│   │   └── Service { name: "svc" } <-- main.rs:79
│   ├── Spawn all services <-- 3c
│   │   └── svc.spawn()
│   │       ├── process::fork() <-- 3d
│   │       │   ├── Parent: store child PID <-- main.rs:41
│   │       │   └── Child: continue to exec <-- main.rs:29
│   │       └── process::execve(&self.exec) <-- 3e
│   └── Main reap/respawn loop
│       └── loop <-- main.rs:90
│           ├── process::waitpid(-1, 0) <-- 3f
│           ├── Match exited PID to service <-- main.rs:99
│           ├── Check if respawn enabled <-- main.rs:106
│           └── svc.spawn() again <-- 3g
```

### Location Details

**Location ID 3a: Init Starts**
- **Description:** PID 1 begins execution, first userspace code running
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:55

**Location ID 3b: Mount Tmpfs**
- **Description:** Mount essential filesystems: /tmp, /dev, /ctl
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:62

**Location ID 3c: Spawn Services**
- **Description:** Fork and exec login-manager and svc services
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:87

**Location ID 3d: Fork Child Process**
- **Description:** Service spawn: fork creates child process
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:28

**Location ID 3e: Exec Service Binary**
- **Description:** Child process replaces itself with service executable
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:31

**Location ID 3f: Wait for Child Exit**
- **Description:** Main loop: wait for any child process to exit
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:96

**Location ID 3g: Respawn Service**
- **Description:** If service marked respawn=true, restart it after crash
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\init\src\main.rs:108

---

## Trace 4: Login Manager → Desktop Shell Transition

**Description:** GUI login manager authenticates user, then execs ADE desktop environment to establish graphical session

### Trace Diagram

```
Login Manager Process
├── user_main() entry <-- main.rs:19
│   ├── Window::create() <-- 4a
│   ├── Main event loop <-- main.rs:38
│   │   ├── get_key() - read input <-- main.rs:46
│   │   ├── verify_password() <-- 4b
│   │   │   └── fs::read_to_string() <-- 4c
│   │   │       └── /etc/shadow lookup <-- main.rs:9
│   │   └── On success:
│   │       └── process::execve() <-- 4d
│   │           └── SYS_EXECVE (59) <-- process.rs:268
│   │               └── Kernel replaces process
│   │                   └── Load /bin/ade ELF
│   └── render login UI <-- main.rs:228
└── Process replaced by ADE

ADE Desktop Environment (same PID)
├── user_main() entry <-- main.rs:21
│   ├── Window::create() <-- 4e
│   ├── Desktop::new() <-- main.rs:32
│   ├── Event loop <-- 4f
│   │   ├── get_key() / get_mouse() <-- main.rs:48
│   │   ├── desktop.tick() <-- main.rs:46
│   │   ├── desktop.handle_event() <-- main.rs:55
│   │   └── render::render() <-- 4g
│   │       └── compositor.composite()
│   └── Session running
```

### Location Details

**Location ID 4a: Create Login Window**
- **Description:** Login manager creates GUI window for authentication
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\login-manager\src\main.rs:21

**Location ID 4b: Verify Credentials**
- **Description:** Check username/password against /etc/shadow
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\login-manager\src\main.rs:54

**Location ID 4c: Read Shadow File**
- **Description:** Load password hashes from /etc/shadow
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\login-manager\src\main.rs:11

**Location ID 4d: Exec Desktop Environment**
- **Description:** Replace login-manager process with ADE desktop shell
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\login-manager\src\main.rs:55

**Location ID 4e: ADE Creates Desktop Window**
- **Description:** Desktop environment initializes main window
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:24

**Location ID 4f: Desktop Event Loop**
- **Description:** Main desktop loop: process input, update state, render
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:45

**Location ID 4g: Render Desktop Frame**
- **Description:** Composite and render desktop UI to framebuffer
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\ade\src\main.rs:81

---

## Trace 5: Userspace Binary Entry Point (_start)

**Description:** Common entry mechanism for all userspace binaries: _start receives stack, initializes args, calls main, exits

### Trace Diagram

```
Userspace Binary Entry Flow
└── Kernel sets up user stack & jumps to _start
    └── _start(stack) entry point <-- 5a
        ├── args::init(stack) <-- 5b
        │   ├── Read argc from *stack <-- 5c
        │   └── Store argv pointer (stack+8) <-- args.rs:9
        ├── Call main() <-- 5d
        │   └── sarga_main! macro wrapper <-- 5e
        │       └── user_main() implementation <-- lib.rs:61
        │           └── (user code executes)
        └── process::exit(code) <-- 5f
            └── SYS_EXIT syscall (60) <-- process.rs:77
                └── Kernel terminates process
```

### Location Details

**Location ID 5a: Userspace Entry Point**
- **Description:** All userspace binaries start here after kernel sets up stack
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\start.rs:3

**Location ID 5b: Initialize Arguments**
- **Description:** Parse argc/argv from stack into global state
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\start.rs:4

**Location ID 5c: Read Argc from Stack**
- **Description:** First word on stack is argument count
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\args.rs:7

**Location ID 5d: Call User Main**
- **Description:** Invoke user-defined main function (from sarga_main! macro)
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\lib.rs:61

**Location ID 5e: Macro Wrapper**
- **Description:** sarga_main! macro expands to call user_main and handle exit
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\lib.rs:61

**Location ID 5f: Process Exit**
- **Description:** SYS_EXIT syscall terminates process and returns to kernel
- **Path:** c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\process.rs:77

---

## Code Snippets

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

### File: c:\Users\nanda\Desktop\Github\SkyOS\libsarga\src\process.rs

```rust
Lines: 264-268
    envp.push(core::ptr::null());

    let r = unsafe {
        syscall3(
            59,
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
