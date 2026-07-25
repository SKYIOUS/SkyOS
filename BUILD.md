# SkyOS Build System

## Single Entry Point

`build_disk.py` is the unified build script for SkyOS. It handles the entire build pipeline from userspace compilation to disk image creation.

## Quick Start

```bash
# Full build (userspace → kernel → UEFI image → VDI)
python build_disk.py

# Kernel-only build (faster for kernel development)
python build_disk.py --kernel-only

# Userspace-only build
python build_disk.py --userspace-only

# Build with ISO output
python build_disk.py --iso --version 0.6.0

# Build in release mode
python build_disk.py --release
```

## Build Options

| Option | Description |
|--------|-------------|
| `--kernel-only` | Only build kernel and UEFI bootimage (skips userspace) |
| `--userspace-only` | Only build userspace binaries and initrd (skips kernel) |
| `--no-vdi` | Skip VirtualBox VDI conversion |
| `--iso` | Create bootable ISO image |
| `--version VERSION` | Version string for ISO output (default: 0.6.0) |
| `--release` | Build in release mode with optimizations |

## Build Pipeline

The full build pipeline (`python build_disk.py`) performs the following steps:

1. **Userspace Build**: Compiles all userspace binaries for `x86_64-sarga` target
2. **Initrd Creation**: Creates `initrd.tar` with FHS directory structure
3. **Kernel Build**: Builds kernel with bootloader integration
4. **Bootimage Creation**: Creates UEFI bootimage (`skyos_uefi.img`)
5. **VDI Conversion**: Converts to VirtualBox VDI format (optional)
6. **ISO Creation**: Creates bootable ISO image (optional with `--iso`)

## Build Outputs

- `skyos_uefi.img` - UEFI bootable disk image
- `skyos.vdi` - VirtualBox disk image (if not skipped)
- `release/skyos-<version>.iso` - Bootable ISO (if `--iso` specified)
- `initrd.tar` - Initial ramdisk with userspace

## Running

### QEMU

```bash
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=skyos_uefi.img -m 512M -smp 2
```

### VirtualBox

Use `skyos.vdi` with EFI enabled in System > Motherboard settings.

### ISO

```bash
qemu-system-x86_64 -bios OVMF.fd -cdrom release/skyos-<version>.iso -m 512M -smp 2
```

## Prerequisites

- **Rust nightly** with `rust-src` and `llvm-tools-preview` components
- **Python 3** for build scripts
- **QEMU** for testing (optional)
- **VirtualBox** with VBoxManage for VDI conversion (optional)
- **xorriso** for ISO creation (optional, can use WSL on Windows)

## Development Workflow

For kernel development, use `--kernel-only` to skip userspace compilation:

```bash
python build_disk.py --kernel-only
```

For userspace development, use `--userspace-only`:

```bash
python build_disk.py --userspace-only
```

## Legacy Scripts

The following scripts have been removed and their functionality consolidated into `build_disk.py`:

- `build.ps1` - Use `python build_disk.py` instead
- `make_bootimage.ps1` - Use `python build_disk.py --kernel-only` instead
- `make_installer_iso.py` - Use `python build_disk.py --iso` instead
- `build_userspace.ps1` - Use `python build_disk.py --userspace-only` instead
- `rebuild_initrd.ps1` - Use `python build_disk.py --userspace-only` instead

## Troubleshooting

### Kernel directory not found

The build expects the kernel to be in a `kernel/` subdirectory or the separate SKYIOUS KERNEL repo. Ensure the kernel is available at the expected location.

### VBoxManage not found

If VDI conversion fails, install VirtualBox or use `--no-vdi` to skip this step.

### xorriso not found

ISO creation requires xorriso. On Windows, it can be accessed via WSL. If unavailable, omit the `--iso` flag.
