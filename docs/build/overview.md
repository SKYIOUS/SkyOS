# Build System Overview

SkyOS builds with Rust: a userspace Cargo workspace plus an external kernel crate, a builder crate
that produces the UEFI image, and Python scripts that orchestrate the pieces.

## Build Architecture

1. **Userspace compilation**: the workspace is compiled for the custom `x86_64-sarga.json` target
   (`cargo build --target x86_64-sarga.json [--release]`)
2. **Initrd assembly**: `build_initrd.py` packs userspace binaries into `initrd.tar`
3. **Kernel compilation**: the kernel crate builds for `x86_64-unknown-none` with
   `-C target-feature=-mmx,-sse,+soft-float` and the `linker.ld` script
4. **Boot image creation**: `kernel/builder` (via `bootloader::UefiBoot`) combines the kernel ELF
   and initrd into `bootimage-vahi_kernel.bin`
5. **(Optional) VDI/ISO**: `build_disk.py` converts the image to a VirtualBox VDI, and
   `scripts/make_iso.py` produces a UEFI-bootable ISOHybrid

## Build Commands

| Command | Description |
|---------|-------------|
| `python build_disk.py` | Full build (userspace → kernel → UEFI image → VDI) |
| `python build_disk.py --kernel-only` | Kernel + UEFI image only |
| `python build_disk.py --iso --version X` | Full build + ISO |
| `make build` | Debug userspace build |
| `make build-release` | Release userspace build |
| `make kernel` | Kernel release build |
| `make initrd` | Build initrd.tar |
| `make bootimage` | Build the UEFI boot image |
| `make iso` | Build the ISO |
| `make run` / `make run-nographic` | Boot the ISO in QEMU |
| `make qemu-test` | Boot in QEMU and assert a login prompt |
| `cargo clippy --target x86_64-sarga.json -- -D warnings` | Lint (all warnings are errors) |

## Build Outputs

- `target/x86_64-sarga/{debug,release}/` — userspace binaries
- `kernel/kernel/target/x86_64-unknown-none/{debug,release}/vahi_kernel` — kernel ELF
- `kernel/target/x86_64-vahi/{debug,release}/bootimage-vahi_kernel.bin` — UEFI bootable image
- `initrd.tar` — initial ramdisk
- `release/skyos-<version>.iso` — optional ISO
- `skyos_uefi.img` — QEMU raw disk image (VDI-convertible)

## Build Configuration

- Cargo features (`kernel/kernel/Cargo.toml`): `smp`, `net`, `ai_rule`, `ai_llm`, `ash`,
  `hypervisor`, `ext4` (kernel builds use `net,smp,ai_rule,ext4` by default)
- Environment variables: `VAHI_ARCH` (x86_64/aarch64), `QEMU_MEM`, `QEMU_SMP`
- `--version <v>` / `--iso` flags on `build_disk.py`
