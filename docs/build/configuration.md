# Feature Flags and Build Configuration

SkyOS uses Cargo features for build-time configuration. The kernel manifest is `kernel/kernel/Cargo.toml`; the userspace workspace is the root `Cargo.toml`.

## Kernel Features (`kernel/kernel/Cargo.toml`)

```toml
[features]
default = ["smp", "net", "ai_rule", "ext4"]
verification = []
ai_rule = []
ai_llm = []
smp = []
net = []
uhci = []
ext4 = []
self_test = []
ash = []
gpu = []
hypervisor = []
objects_v2 = []
```

- `smp` — SMP support (AP boot, per-CPU schedulers)
- `net` — smoltcp networking (socket syscalls return `ENOSYS` when off)
- `ai_rule` — Vahiai rule engine (default); `ai_llm` — LLM support
- `ext4` — ext4 read-only filesystem
- `self_test` — boot-time TAP self-tests to serial (CI gate)
- `ash` — ASh language syscalls (310–313); `hypervisor` — hypervisor syscalls (340–349)
- `uhci`, `gpu`, `objects_v2`, `verification` — optional subsystems

## Enabling Features

```bash
# Kernel, with self-tests for CI
cargo build --release --target x86_64-unknown-none \
    -Zbuild-std=core,alloc --features net,smp,ai_rule,self_test

# Userspace workspace
cargo build -Zbuild-std=core,alloc --target x86_64-sarga.json
```

## Kernel Command-Line Parameters

The bootloader passes no configurable log-level/init/root/mem parameters — the kernel reads the `BootInfo` from `bootloader` v0.11 and mounts the initrd-provided root filesystem (tarfs) with a fixed `/sbin/init`-style bootstrap. There is no `log_level` parameter; serial logging is unconditional via `serial_write`.

## Build Profiles

Debug builds (`profile.dev`):
- `panic = "abort"`, stack-protector `-Z stack-protector=strong`
- Unoptimized, full debug symbols

Release builds (`profile.release`):
- `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, stack-protector
- Target rustflags (`.cargo/config.toml`): `-C target-feature=-mmx,-sse,+soft-float`, `-C link-arg=-Tlinker.ld`, `-C relocation-model=static`

There is no KASAN, no `profiling`/`log_*` feature flags, and no PGO in the kernel build.
