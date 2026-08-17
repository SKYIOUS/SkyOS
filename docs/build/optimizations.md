# Build Optimization Flags and LTO

SkyOS uses several optimization techniques for release builds.

## Link-Time Optimization (LTO)

LTO is enabled in release builds for the kernel (`kernel/kernel/Cargo.toml`):

```toml
[profile.release]
opt-level = "z"   # size-optimized
lto = true        # thin LTO
codegen-units = 1
panic = "abort"
```

## Code Generation

```toml
[profile.release]
opt-level = "z"
debug = false
panic = "abort"
```

- **opt-level = "z"**: Maximum optimization for size
- **panic = "abort"**: No unwinding code, reduces binary size
- **lto = true / codegen-units = 1**: Cross-crate inlining at the cost of compile time

## Target-Specific Optimizations

The kernel's flags live in `kernel/kernel/.cargo/config.toml`:

```toml
[target.x86_64-unknown-none]
rustflags = [
    "-C", "target-feature=-mmx,-sse,+soft-float",  # SIMD not saved on ctx switch
    "-C", "link-arg=-Tlinker.ld",                  # Custom linker script
    "-C", "relocation-model=static",
]
```

## Binary Size Reduction

```bash
# Strip debug symbols from the kernel ELF
strip -s kernel/kernel/target/x86_64-unknown-none/release/vahi_kernel

# Remove unused code
rustflags = ["-C", "link-arg=--gc-sections"]
```

## Profile-Guided Optimization (PGO)

For maximum performance, PGO can be used:

```bash
# Step 1: Build with instrumentation
RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --target x86_64-sarga.json --release

# Step 2: Run representative workloads in QEMU
make run

# Step 3: Rebuild using profiling data
RUSTFLAGS="-Cprofile-use=/tmp/pgo-data" cargo build --target x86_64-sarga.json --release
```

## Compile-Time Optimization

For faster compilation during development:

```toml
[profile.dev]
opt-level = 0
debug = true
codegen-units = 256   # More parallel codegen
incremental = true    # Incremental compilation
```
