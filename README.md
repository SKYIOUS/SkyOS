# SKYIOUS + SARGA OS

> A modern, GPL-free operating system userland built on the SARGA kernel.
> Fast, lightweight, self-hosted, and extensible.

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-nightly-dea584?logo=rust&logoColor=fff)](https://www.rust-lang.org)
[![Target](https://img.shields.io/badge/target-x86__64%20%7C%20aarch64-blueviolet)](#)
[![License: SSL](https://img.shields.io/badge/license-SSL-green)](#)
[![Custom Target](https://img.shields.io/badge/no__std-custom%20target-critical)](#)

</div>

---

## Table of Contents

- [Overview](#overview)
- [How This Project Was Built](#how-this-project-was-built)
- [Architecture](#architecture)
- [Application Suite](#application-suite)
- [Core Utilities](#core-utilities)
- [Library: libsarga](#library-libsarga)
- [Building from Source](#building-from-source)
- [Running in QEMU](#running-in-qemu)
- [Project Structure](#project-structure)
- [Configuration](#configuration)
- [Contributing](#contributing)
- [License](#license)

---

## Overview

**SARGA OS** is the userspace environment of SARGA OS -- a complete operating system userland built entirely in Rust, targeting the custom `x86_64-sarga` and `aarch64-sarga` targets. It provides:

- A full **GUI desktop environment** with window manager, compositing, and widget toolkit
- **62+ core utilities** covering all standard Unix command-line operations
- A **shell** (`sash`) with scripting, job control, readline, and pipeline support
- A **package manager** (`spkg`) with dependency resolution
- **Networking tools**: HTTP client, DNS resolver, netcat, echo server
- **AI integration** via the SARGAAI kernel subsystem
- **Multi-architecture**: x86_64 and aarch64 userspace targets

SARGA OS runs on top of the **SARGA kernel** -- a monolithic Rust kernel with 90+ syscalls, 7 filesystems, 12+ drivers, networking, eBPF, and GUI compositor.

## How This Project Was Built
>
> The vast majority of this codebase was generated with the assistance of AI (large language models).
> This allowed a single developer to create an entire OS userland from scratch — something that
> would normally require a team of engineers over many years.
>
> **We are looking for human contributors.** If you understand Rust, operating systems, or any part
> of this codebase — whether you wrote none of it or all of it — your help is needed and welcome.
> We are actively seeking people to:
>
> - Review the code for correctness, security, and performance
> - Fix bugs, edge cases, and incomplete implementations
> - Refactor AI-generated code into idiomatic, maintainable Rust
> - Add tests, documentation, and missing features
> - Help transition this from an AI-driven prototype to a community-maintained project
>
> No contribution is too small. Open an issue, submit a PR, or start a discussion.
> See [CONTRIBUTING.md](CONTRIBUTING.md) for details.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Userspace (SARGA OS)                     │
│                                                         │
│  ┌─────────────┐ ┌──────────┐ ┌────────────────────┐   │
│  │  sash (sh)  │ │ coreutils│ │  ADE Desktop       │   │
│  │  scripting  │ │ 62 utils │ │  Window Manager     │   │
│  └─────────────┘ └──────────┘ └────────────────────┘   │
│  ┌─────────────┐ ┌──────────┐ ┌────────────────────┐   │
│  │  spkg (pkg) │ │ nettools │ │  GUI Applications  │   │
│  │  dependency │ │ curl, nc │ │  skyedit, skyfiles │   │
│  └─────────────┘ └──────────┘ └────────────────────┘   │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │               libsarga (std lib)                │    │
│  │  syscalls | GUI | FS | net | thread | posix    │    │
│  └─────────────────────────────────────────────────┘    │
├─────────────────────────────────────────────────────────┤
│                     System Calls                         │
│         (90+ Linux-compatible syscalls via `syscall`)    │
├─────────────────────────────────────────────────────────┤
│                   SARGA Kernel                            │
│  scheduler | VFS | drivers | network | SkyFS | eBPF     │
└─────────────────────────────────────────────────────────┘
```

### Library Stack

| Layer | Description |
|-------|-------------|
| **Applications** | Desktop apps, utilities, tools |
| **libsarga** | Core no_std library: syscall wrappers, GUI toolkit, I/O, networking, threading, POSIX compat |
| **Syscall ABI** | Frozen v1.0 ABI with Linux-compatible numbering |
| **SARGA Kernel** | Monolithic Rust kernel providing all OS services |

All userspace binaries are compiled with `-C panic=abort`, custom linker scripts, and position-independent code with full RELRO.

---

## Application Suite

### Desktop Environment (ADE)

The **Application Desktop Environment** provides a full graphical workspace with window management, compositing, mouse/keyboard input, and desktop widgets.

| Component | Description |
|-----------|-------------|
| **Window Manager** | Compositing WM with title bars, minimize/close, hover effects, drag-to-move, resize |
| **Taskbar** | Application launcher, window list, system clock |
| **Notification System** | Toast notifications with Info/Warning/Error types, click-to-dismiss |
| **Desktop Icons** | Application shortcuts on the desktop background |
| **Theming** | Dark theme with configurable accent colors |

### GUI Applications

| Application | Description |
|-------------|-------------|
| **sarga-term** | Terminal emulator with full graphical rendering, scrollback, clipboard support |
| **skyedit** | Text editor with syntax highlighting code editing |
| **skyfiles** | File manager with directory navigation, file operations, thumbnail previews |
| **skyview** | Image viewer supporting PNG format |
| **calculator** | Desktop calculator with basic arithmetic operations |
| **skysettings** | Settings panel for system configuration |
| **login-manager** | Graphical login/display manager |

### Widget Toolkit (`libsarga`)

A complete GUI widget library, similar to the Windows Forms / Qt model:

```
Widgets:  Button  Label  TextBox  CheckBox  ComboBox
          Slider  Scrollbar  ProgressBar  TabWidget
Layout:   HBox  VBox  Grid  StackPanel
Others:   MenuBar  Dialog  Theme  PngDecoder
```

---

## Core Utilities

<div align="center">

| Category | Commands |
|----------|----------|
| **File Operations** | `cat` `ls` `echo` `mkdir` `rm` `cp` `mv` `touch` `ln` `readlink` `tee` `which` `basename` `dirname` `find` |
| **Text Processing** | `grep` `sed` `awk` `cut` `tr` `uniq` `diff` `patch` `sort` `wc` `head` `nl` `tac` |
| **System Info** | `ps` `top` `df` `du` `free` `uptime` `stat` `uname` `date` `hostname` `id` `whoami` |
| **Access Control** | `login` `passwd` `su` `chmod` `chown` |
| **Archiving** | `tar` `gzip` |
| **Networking** | `ping` `curl` `ifconfig` `nc` `resolve` |
| **Binary Tools** | `hexdump` `od` `dd` `lspci` |
| **Filesystem** | `mkfs_skyfs` `mount` `umount` `sync` |
| **Process Control** | `kill` `sleep` `true` `false` `env` `xargs` `watch` |

</div>

---

## Library: libsarga

`libsarga` is the standard library for SARGA OS userspace. It is a `no_std` library that directly interfaces with the kernel through syscalls.

### Modules

| Module | Description |
|--------|-------------|
| `syscall` | Raw syscall wrappers (inline assembly) |
| `io` | File I/O: open, read, write, close, seek, stat, readdir |
| `process` | Process management: fork, execve, wait4, exit, kill |
| `thread` | Threading: clone, futex, TLS, thread-local storage |
| `net` | Networking: socket, bind, connect, send, recv, DNS |
| `gui` | GUI syscalls: window creation, buffer flush, input events |
| `fs` | Filesystem helpers: mkfs, mount info |
| `gpu` | GPU/DRM control: display info, dumb buffers, page flip |
| `posix` | POSIX C-compatible wrappers: open/read/write/mmap/fork/execve |
| `hash` | Cryptographic hashing: SHA-256, PBKDF2 |
| `png` | PNG image decoding (via miniz_oxide) |
| `vahiai` | AI subsystem interface |
| `theme` | GUI theming engine |
| `widget` | Base widget trait and event handling |
| `button` | Push button widget |
| `label` | Text label widget |
| `textbox` | Text input widget |
| `checkbox` | Checkbox widget |
| `combobox` | Dropdown selection widget |
| `slider` | Range slider widget |
| `scrollbar` | Scroll bar widget |
| `progress_bar` | Progress bar widget |
| `tab_widget` | Tabbed panel widget |
| `menubar` | Menu bar widget |
| `dialog` | Dialog box widget |
| `layout` | Layout containers (HBox, VBox, Grid) |

### POSIX Compatibility Layer

The `posix` module provides C-compatible function signatures for porting software:

```rust
// Example: POSIX-style wrappers in libsarga
pub unsafe extern "C" fn open(path: *const u8, flags: i32, mode: u32) -> i32;
pub unsafe extern "C" fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
pub unsafe extern "C" fn write(fd: i32, buf: *const u8, count: usize) -> isize;
pub unsafe extern "C" fn mmap(...) -> *mut u8;
pub unsafe extern "C" fn fork() -> i32;
pub unsafe extern "C" fn execve(path: *const u8, argv: *const *const u8, envp: *const *const u8) -> i32;
// ... 40+ additional POSIX wrappers
```

Shell: **sash** (Sarga Shell)

`sash` is a full-featured Unix shell with:

```
- Command parsing with pipelines and I/O redirection
- Environment variable expansion ($VAR, ${VAR}, $?)
- Job control (bg, fg, jobs)
- Readline with history and tab completion
- Shell scripting (conditionals, loops, functions)
- Aliases and shell functions
- PATH searching
- Korn language bridge
- Background job management
```

---

## Building from Source

### Prerequisites

- **Rust nightly** (rustup + rust-src component)
- **LLVM tools** (llvm-tools-preview)
- **Custom target specs** (included in the repo)

### Quick Build

```bash
# Build all userspace binaries
./build.sh all

# Or using PowerShell on Windows
.\build.ps1 all

# Build a specific component
cargo build --target x86_64-sarga.json --release -p sash
cargo build --target x86_64-sarga.json --release -p coreutils
cargo build --target x86_64-sarga.json --release -p ade
```

### Release Build

```powershell
# Full release pipeline: build, stage, initrd, kernel rebuild, disk image
.\scripts\release_build.ps1
```

### Build Outputs

All binaries are compiled for the custom `x86_64-sarga` target with:
- `-C panic=abort` (no unwinding)
- `-C target-feature=-mmx,-sse,+soft-float` (no SIMD)
- Position-independent executables (PIE) with full RELRO
- LTO + opt-level=3 for release builds

```
target/x86_64-sarga/release/
├── init                  # PID 1 init process
├── sash                  # Shell
├── ade                   # Desktop environment
├── coreutils/            # 62 utility binaries
├── net-tools/            # 5 networking utilities
├── spkg                  # Package manager
├── sarga-term            # Terminal emulator
├── skyedit               # Text editor
├── calculator            # Calculator
├── skyfiles              # File manager
├── skyview               # Image viewer
├── skysettings           # Settings panel
├── skyd                  # System daemon
├── login-manager         # Login manager
├── aicli                 # AI CLI
├── proc                  # Process monitor
└── libsarga.so           # Standard library
```

---

## Running in QEMU

SARGA OS runs on top of the SARGA kernel. The typical workflow:

```bash
# 1. Build userspace (this repo)
./build.sh all

# 2. Build kernel + bootimage (in the kernel repo)
cd ../SARGIUOUS\ KERNEL
./make_bootimage.sh

# 3. Run in QEMU
cd SARGA-OS
make run
```

Or use the development loop:

```powershell
# Windows development loop
.\scripts\dev_loop.ps1
```

QEMU configuration (from `Makefile`):

```
- UEFI boot (OVMF)
- 512 MB RAM
- 2 CPU cores
- AHCI disk controller
- Intel E1000 NIC (user-mode networking)
- VGA display
```

---

## Project Structure

```
SARGA-OS/
├── .cargo/                 # Cargo configuration (build-std, rustflags)
├── libsarga/               # Core standard library (no_std)
│   └── src/
│       ├── lib.rs          # Library root with module declarations
│       ├── io.rs           # File I/O syscalls
│       ├── process.rs      # Process management
│       ├── thread.rs       # Threading (clone, futex, TLS)
│       ├── syscall.rs      # Raw syscall wrappers
│       ├── net.rs          # Networking
│       ├── gui.rs          # GUI syscall interface
│       ├── fs.rs           # Filesystem helpers
│       ├── gpu.rs          # GPU/DRM control
│       ├── posix.rs        # POSIX C-compatible wrappers
│       ├── png.rs          # PNG decoder
│       ├── hash.rs         # Cryptographic hash (SHA-256, PBKDF2)
│       ├── vahiai.rs       # AI subsystem interface
│       ├── theme.rs        # GUI theming
│       ├── widget.rs       # Base widget
│       ├── button.rs       # Push button
│       ├── label.rs        # Text label
│       ├── textbox.rs      # Text input
│       ├── checkbox.rs     # Checkbox
│       ├── combobox.rs     # Dropdown
│       ├── slider.rs       # Range slider
│       ├── scrollbar.rs    # Scroll bar
│       ├── progress_bar.rs # Progress bar
│       ├── tab_widget.rs   # Tab panel
│       ├── menubar.rs      # Menu bar
│       ├── dialog.rs       # Dialog box
│       └── layout.rs       # Layout containers
├── sash/                   # Sarga Shell
│   └── src/
│       ├── main.rs
│       ├── parser.rs       # Command parser
│       ├── executor.rs     # Command executor
│       ├── builtins.rs     # Built-in commands
│       ├── readline.rs     # Readline/history
│       └── scripting.rs    # Shell scripting
├── init/                   # Init process (PID 1)
├── ade/                    # Desktop environment
├── coreutils/              # 62 Unix utilities
│   └── src/
│       ├── cat.rs          # Concatenate files
│       ├── ls.rs           # List directory
│       ├── grep.rs         # Text search
│       ├── ps.rs           # Process list
│       ├── ping.rs         # ICMP echo
│       ├── mkfs_skyfs.rs   # Create SkyFS
│       ├── mount.rs        # Mount filesystems
│       └── ...             # 55+ more utilities
├── spkg/                   # Package manager
├── nettools/               # Networking tools
├── aicli/                  # AI command-line interface
├── proc/                   # Process monitor daemon
├── sarga-term/             # Terminal emulator
├── skyedit/                # Text editor
├── calculator/             # Calculator
├── skyfiles/               # File manager
├── skyview/                # Image viewer
├── skysettings/            # Settings panel
├── skyd/                   # System daemon
├── login-manager/          # Login manager
├── scripts/                # Build and dev automation
│   ├── make_sarga_image.py # End-to-end image creator
│   ├── release_build.ps1   # Production release build
│   ├── dev_loop.ps1        # Fast development loop
│   ├── setup_dev.ps1       # Dev environment setup
│   └── ...
├── staging/                # Initrd staging directory
│   ├── bin/                # Deployed binaries
│   ├── etc/                # System configuration
│   │   ├── init.cfg        # Legacy init config
│   │   └── init.toml       # TOML-based init config
│   └── usr/share/fonts/    # System fonts
├── fonts/                  # Source fonts
├── disk/                   # Disk image creation
├── x86_64-sarga.json       # x86_64 target specification
├── aarch64-sarga.json      # aarch64 target specification
├── sarga.ld                # x86_64 linker script
├── aarch64-sarga.ld        # aarch64 linker script
└── Cargo.toml              # Workspace definition (19 crates)
```

---

## Configuration

### Init System (`/etc/init.toml`)

```toml
hostname = "sarga-os"

[[service]]
name = "login"
exec = "/bin/login-manager"
respawn = true

[[service]]
name = "proc"
exec = "/bin/proc"
respawn = true
```

The init process (PID 1) reads this configuration, spawns services, monitors their health, and respawns them on failure (up to 5 retries).

### Network Configuration

```toml
# /etc/init.d/network.toml
[[interface]]
name = "eth0"
dhcp = true
```

---

## Package Manager: spkg

`spkg` manages native `.skp` packages:

```
Usage: spkg <command> [options]

Commands:
  install <package>    Install a package (with dependency resolution)
  remove <package>     Remove a package
  update               Update package list from repositories
  upgrade              Upgrade all installed packages
  search <query>       Search for packages
  list                 List installed packages
```

Package format: `.skp` (magic `SKYPKG01`, INI manifest, TLV payload).

---

## Target Specifications

### x86_64-sarga.json

```json
{
  "arch": "x86_64",
  "cpu": "x86-64",
  "os": "sarga",
  "env": "",
  "vendor": "sarga",
  "features": "-mmx,-sse,-sse2,-sse3,-ssse3,-sse4.1,-sse4.2,-avx,-avx2,+soft-float",
  "linker-flavor": "ld.lld",
  "linker": "rust-lld",
  "executables": true,
  "position-independent-executables": true,
  "panic-strategy": "abort",
  "relocation-model": "static",
  "pre-link-args": { "ld.lld": ["-T", "sarga.ld"] },
  "crt-objects-fallback": "false"
}
```

### aarch64-sarga.json

Same structure targeting `aarch64-unknown-none` with the `aarch64-sarga.ld` linker script.

---

## Contributing

Contributions are welcome under the **SKYIOUS Software License (SSL)** terms.

### Development Workflow

```powershell
# 1. Build the userspace
.\build.ps1 all

# 2. Run the fast dev loop (builds userspace + kernel + boots in QEMU)
.\scripts\dev_loop.ps1
```

### Coding Standards

- Rust 2021 edition, nightly channel
- `no_std` for all crates (no libstd)
- `#![deny(warnings)]` in kernel-facing code
- Follow existing patterns for syscall wrappers
- Document all public API surface

---

## License

**SKYIOUS Software License (SSL) v1.0**

Copyright (c) 2026 SARGA OS Contributors

A file-level copyleft license that balances freedom for users with protection for the original project. See the [LICENSE](LICENSE) file for full terms.

Key provisions:
- Commercial use permitted with attribution
- Attribution required in documentation and UI
- File-level copyleft (modified files must share-alike)
- Patent grant included
- Optional Maintainer Right clause for upstream fork incorporation
- 30-day cure period for license violations
- "or any later version" compatibility
