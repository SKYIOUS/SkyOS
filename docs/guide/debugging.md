# Debugging Techniques

SkyOS supports multiple debugging methods for kernel development.

## Serial Output

The kernel logs diagnostic information through a serial port (COM1 at I/O port `0x3F8`). Use QEMU's `-serial stdio` to view output. Kernel messages go through `serial_write()`/`println!`.

```rust
// In kernel code
crate::println!("Memory manager initialized: {} frames free", free_frames);
crate::serial_write("[BOOT] page tables ready\n");
```

## GDB Debugging

Connect GDB to QEMU for full breakpoint and step debugging:

```gdb
(gdb) target remote :1234
(gdb) add-symbol-file kernel/kernel/target/x86_64-unknown-none/debug/vahi_kernel 0x200000
(gdb) break kernel_main
(gdb) continue
```

## Kernel Panic Handler

The `#[panic_handler]` in `main.rs` prints:
- `=== KERNEL PANIC ===` and the panic message
- The source file:line of the panic (`info.location()`)
- A boot trace (`boot::with_trace`) listing init events and init-path searches when available
- A stack trace (`debug::print_stack_trace`)
- CR2 (page-fault address), then halts

The kernel also uses `-Z stack-protector=strong` (`__stack_chk_guard`/`__stack_chk_fail` prints "PANIC: Stack smashing detected!").

## QEMU Logging

QEMU can log guest interactions:
```bash
qemu-system-x86_64 -d int,cpu_reset -D qemu.log
```

## Memory Debugging

There is no KASAN. Debugging memory issues relies on:
- The buddy/slab allocator accounting (see `memory/buddy.rs`, `memory/slab.rs`)
- Guard pages on thread stacks
- The kernel `self_test` feature's TAP assertions for allocator invariants
