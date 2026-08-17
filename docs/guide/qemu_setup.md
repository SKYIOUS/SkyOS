# QEMU Setup and Debugging

SkyOS uses QEMU (with OVMF firmware) as the primary emulation and debugging platform.

## Basic QEMU Configuration

```bash
qemu-system-x86_64 \
    -bios OVMF.fd \
    -drive format=raw,file=skyos_uefi.img \
    -m 512M -smp 2 \
    -serial stdio \
    -device e1000,netdev=net0 -netdev user,id=net0
```

Or boot the ISO directly:

```bash
qemu-system-x86_64 \
    -bios OVMF.fd \
    -cdrom release/skyos-<version>.iso \
    -m 512M -smp 2 \
    -serial mon:stdio \
    -nographic
```

`make run` and `make run-nographic` wrap these commands (see `docs/build/building.md`).

## GDB Debugging

Start QEMU with the `-s` flag, then attach GDB to the kernel ELF:

```bash
qemu-system-x86_64 -bios OVMF.fd -drive format=raw,file=skyos_uefi.img -m 512M -smp 2 -serial stdio -s
```

```bash
gdb kernel/kernel/target/x86_64-unknown-none/debug/vahi_kernel
(gdb) target remote :1234
(gdb) break kernel_main
(gdb) continue
```

## Networking

QEMU's user-mode networking provides NAT and port forwarding: `-nic user,hostfwd=tcp::8080-:80`
forwards host port 8080 to guest port 80. The Makefile QEMU targets attach an e1000 device.

## QEMU Monitor

Press `Ctrl+Alt+2` in the QEMU window to access the monitor. Useful commands:
- `info registers` - CPU register dump
- `info cpus` - CPU state
- `system_reset` - Reset the emulated system
- `quit` - Exit QEMU
