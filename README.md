# SkyOS

> A modern operating system userland built in Rust on the Vahi kernel.
> Desktop environment, shell, 60+ coreutils, package manager, networking.

<div align="center">

[![Rust](https://img.shields.io/badge/Rust-nightly-dea584?logo=rust&logoColor=fff)](https://www.rust-lang.org)
[![Target](https://img.shields.io/badge/target-x86__64--sarga-blueviolet)]()
[![CI](https://github.com/SKYIOUS/SKYOS/actions/workflows/ci.yml/badge.svg)](https://github.com/SKYIOUS/SKYOS/actions/workflows/ci.yml)
[![License: SSL](https://img.shields.io/badge/license-SSL-green)]()

</div>

---

## Quick Start

```bash
# Build everything
python build_disk.py --iso

# Boot in QEMU
qemu-system-x86_64 -bios OVMF.fd -cdrom release/skyos-0.6.0.iso -m 512M -smp 2

# Or use the Makefile (WSL/Linux)
make run
```

See [BUILD.md](BUILD.md) for full build options.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Userspace (SkyOS)                     │
│  sash (shell) | coreutils (60+ utils) | ADE Desktop     │
│  spkg (pkg mgr) | nettools | GUI apps                   │
│  ┌─────────────────────────────────────────────────┐     │
│  │              libsarga (std library)              │     │
│  │  syscalls | GUI | FS | net | thread | posix     │     │
│  └─────────────────────────────────────────────────┘     │
├─────────────────────────────────────────────────────────┤ │
│                     System Calls (90+)                   │
├─────────────────────────────────────────────────────────┤ │
│                   Vahi Kernel (separate repo)             │
└─────────────────────────────────────────────────────────┘
```

## Components

| Component | Description |
|-----------|-------------|
| **libsarga** | no_std standard library: syscall wrappers, GUI toolkit, I/O, networking, threading |
| **sash** | Unix shell with scripting, job control, readline, pipelines |
| **coreutils** | 60+ Unix utilities (ls, cat, grep, sed, ps, ping, etc.) |
| **ADE** | Desktop environment with window manager, taskbar, notifications |
| **spkg** | Package manager with dependency resolution |
| **nettools** | HTTP client, DNS resolver, netcat, echo server |
| **init** | PID 1 init process with service management |

### GUI Applications
sarga-term (terminal), sargaedit (text editor), sargafiles (file manager), sargaview (image viewer), calculator, clock, calendar, notes, paint, sysinfo, sysmon, skysettings, login-manager

## Project Structure

| Path | Purpose |
|------|---------|
| `libsarga/` | Standard library (no_std) |
| `sash/` | Shell |
| `init/` | Init process (PID 1) |
| `ade/` | Desktop environment |
| `coreutils/` | 60+ Unix utilities |
| `spkg/` | Package manager |
| `nettools/` | Networking tools |
| `scripts/` | Build and dev automation |
| `.github/workflows/` | CI pipeline definitions |
| `docs/` | Architecture, API, and design docs |

## CI/CD

| Workflow | Trigger | What it does |
|----------|---------|--------------|
| **CI** | Push/PR to main | fmt → clippy → build (debug+release) → cargo-deny → QEMU integration test |
| **Release ISO** | Manual dispatch | Full release build with ISO artifact |

## Documentation

- [Architecture](docs/architecture/overview.md) — kernel design, memory, scheduling, IPC
- [Syscall ABI](docs/SYSCALL_ABI.md) — complete syscall table
- [Build System](BUILD.md) — build options and pipeline
- [Testing](TESTING.md) — testing strategy and infrastructure
- [Contributing](CONTRIBUTING.md) — how to contribute

## License

SKYIOUS Software License (SSL) v1.0 — file-level copyleft with attribution.
See [LICENSE](LICENSE) for full terms.