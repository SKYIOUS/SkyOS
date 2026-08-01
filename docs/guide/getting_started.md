# Getting Started

This guide walks you through setting up the development environment and running SkyOS for the first
time.

## Prerequisites

- Rust nightly toolchain with `rust-src` and `llvm-tools-preview`
- QEMU 6.0+ with OVMF firmware (`OVMF.fd`) for testing
- Python 3.8+ (for build scripts)
- `VBoxManage` for VDI output (optional)

## Quick Start

```bash
# Clone the repository (the kernel lives in an external repo, junctioned at kernel/)
git clone <repo-url>
cd SkyOS

# Full build (userspace → kernel → UEFI image → VDI)
python build_disk.py

# Or build just the kernel + UEFI image
python build_disk.py --kernel-only

# Run in QEMU
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=skyos_uefi.img -m 512M -smp 2
```

## Project Structure

The repository is organized as follows:
- `libsarga/` - Userspace runtime library (syscall wrappers, allocator, net, GUI, …)
- `ade/` - The desktop environment (window manager, services, launcher)
- `coreutils/`, `sash/`, `init/`, `login-manager/`, `passwd/`, `nettools/` - Core userspace programs
- `scripts/` - `build_initrd.py`, `make_iso.py`
- `build_disk.py` - Full-build orchestrator (also converts to VDI)
- `Makefile` - Aggregate targets (`build`, `kernel`, `bootimage`, `iso`, `run`, `qemu-test`)
- `kernel/` - Junction to the external Vahi kernel source (kernel crate + builder crate)
- `x86_64-sarga.json` - Custom userspace target spec

## First Boot

When SkyOS boots, you should see:
1. UEFI bootloader initializes (`bootloader` crate)
2. Kernel logs CPU and memory initialization
3. The async executor starts
4. The init service starts `login-manager`, and a `login:` prompt appears on the serial console
