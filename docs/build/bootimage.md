# UEFI Boot Image Creation

SkyOS boots on UEFI systems using the `bootloader` crate (v0.11), wired up by the `kernel/builder`
crate.

## Boot Process

1. **UEFI firmware** loads the bootloader (`BOOTX64.EFI`) from the ESP
2. **Bootloader** (`bootloader::UefiBoot`) initializes UEFI services, graphics output, and loads
   the kernel ELF plus the initrd (embedded as a ramdisk)
3. **Kernel** is parsed and mapped by the bootloader at its higher-half address
4. **Exit boot services** — UEFI boot services are terminated
5. **Kernel entry** — execution transitions to `kernel_main(BootInfo)`

## Boot Image Structure

`kernel/builder` (run from `kernel/`) produces a disk image whose layout is created by the
bootloader crate (MBR/GPT + FAT ESP containing the UEFI application, kernel, and ramdisk):

```
bootimage-vahi_kernel.bin
├── ESP (FAT32)
│   └── EFI/BOOT/BOOTX64.EFI     # UEFI bootloader + embedded kernel
└── ramdisk (kernel/initrd.tar)
```

## Creating the Boot Image

```bash
# 1. Build the kernel (produces kernel/kernel/target/x86_64-unknown-none/<profile>/vahi_kernel)
cd kernel/kernel && cargo +nightly build

# 2. Build the initrd
cd ../.. && python build_initrd.py

# 3. Run the builder (from kernel/, outputs kernel/target/x86_64-vahi/<profile>/bootimage-vahi_kernel.bin)
cargo run --release --manifest-path kernel/builder/Cargo.toml
```

Or simply: `make bootimage` / `python build_disk.py --kernel-only`.

The builder (`kernel/builder/src/main.rs`) picks the kernel from
`kernel/kernel/target/x86_64-unknown-none/<profile>/vahi_kernel`, attaches
`kernel/initrd.tar` when present, and writes the UEFI image with `bootloader::UefiBoot`.

## Running

```bash
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=skyos_uefi.img -m 512M -smp 2
```

## ISO Creation

`scripts/make_iso.py` builds a UEFI-bootable ISOHybrid (file-based El Torito + MBR + GPT) from the
boot image:

```bash
python build_disk.py --iso --version 0.6.0
# → release/skyos-0.6.0.iso
```
