# Cross-compilation for x86_64

SkyOS is cross-compiled from any host (Windows/Linux/macOS) using Rust and the `bootloader` crate —
there is no GCC cross-toolchain.

## Kernel Target

The kernel builds for the built-in `x86_64-unknown-none` target (no target JSON needed). Flags are
set in `kernel/kernel/.cargo/config.toml`:

```toml
[build]
target = "x86_64-unknown-none"

[target.x86_64-unknown-none]
rustflags = [
    "-C", "target-feature=-mmx,-sse,+soft-float",
    "-C", "link-arg=-Tlinker.ld",
    "-C", "relocation-model=static",
]
```

## Key Settings

- **`-C target-feature=-mmx,-sse,+soft-float`**: SIMD registers are not saved on context switch, so
  they are disabled and the ABI uses soft floats
- **`-Tlinker.ld`**: custom linker script (higher-half at `0xFFFFFFFF80000000`)
- **`relocation-model=static`**: no PIE for the kernel
- **`panic-strategy: abort`** (profiles): no unwinding in kernel space
- `aarch64-unknown-none` has an equivalent stanza for the aarch64 port (`aarch64-linker.ld`,
  `+soft-float,+strict-align`)

## Requirements

- Nightly Rust with `rust-src` (for `-Zbuild-std`) and `llvm-tools-preview`
- `rust-lld` ships with Rust — no separate linker install

## Userspace Target

Userspace uses a **custom target spec** in the repo root: `x86_64-sarga.json`
(`llvm-target: x86_64-unknown-none`, `env: sarga`, soft-float, `-T sarga.ld` via pre-link args,
`panic-strategy: abort`). The entire userspace workspace is built against it:

```bash
cargo build --target x86_64-sarga.json --release
```

All userspace programs link `libsarga` as their runtime (entry point `_start`, PIE ELF).
