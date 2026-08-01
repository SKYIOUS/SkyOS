# How to Build SkyOS

The build is orchestrated by `build_disk.py` or the `Makefile`. It has two halves: the Rust
**userspace workspace** (this repository) and the **kernel** (an external dependency, symlinked/junctioned at
`kernel/`).

## Prerequisites

- **Nightly Rust** with the `rust-src` and `llvm-tools-preview` components (`rustup component add
  rust-src llvm-tools-preview`)
- A custom linker for the kernel: `kernel/linker.ld` (or `aarch64-linker.ld`)
- For VDI output: `VBoxManage` (from VirtualBox)
- For ISO output: `xorriso`/`mkisofs` (via `scripts/make_iso.py`)

## Full Build

```bash
python build_disk.py                          # userspace → kernel → UEFI image → VDI
python build_disk.py --kernel-only            # kernel + UEFI image only (faster)
python build_disk.py --iso --version 0.6.0    # full build + ISO in release/
```

## Makefile Targets

```bash
make build                # cargo build --target x86_64-sarga.json
make build-release        # cargo build --target x86_64-sarga.json --release
make kernel               # kernel: cargo build --release -Zbuild-std=core,alloc --features net,smp,ai_rule,ext4
make initrd               # build_initrd.py → initrd.tar → kernel/SkyOS/
make bootimage            # kernel/builder → bootimage-vahi_kernel.bin
make iso                  # scripts/make_iso.py → release/skyos-<version>.iso
make run / run-nographic  # boot the ISO in QEMU (OVMF, 512M, 2 cpus)
make qemu-test            # boot and assert a login prompt appears
```

## Build Artifacts

- Userspace binaries: `target/x86_64-sarga/{debug,release}/` (per-crate)
- Kernel ELF: `kernel/kernel/target/x86_64-unknown-none/{debug,release}/vahi_kernel`
- UEFI boot image: `kernel/target/x86_64-vahi/{debug,release}/bootimage-vahi_kernel.bin`
- Initrd: `initrd.tar` (copied to `kernel/SkyOS/`)
- ISO: `release/skyos-<version>.iso`

## Running in QEMU

```bash
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=skyos_uefi.img -m 512M -smp 2
qemu-system-x86_64 -bios OVMF.fd -cdrom release/skyos-<version>.iso -m 512M -smp 2 -nographic
```
