# Building Userspace Programs

SkyOS userspace is a Rust workspace. Every program links the `libsarga` runtime and is compiled
for the custom `x86_64-sarga.json` target (see `docs/build/cross_compilation.md`).

## Building

```bash
# Debug (default)
cargo build --target x86_64-sarga.json

# Release (optimized, used for the disk image)
cargo build --target x86_64-sarga.json --release
```

Binaries land in `target/x86_64-sarga/{debug,release}/`. The `Makefile` wraps this as
`make build` / `make build-release`.

## Building the Init System

The init service (`init/`) and the ADE desktop (`ade/`) are regular workspace crates and build with
the same command.

## Including in the Initrd

`build_initrd.py` packs the userspace binaries (init, sash, coreutils, ADE, login-manager, …) into
`initrd.tar`, which the bootloader embeds as the kernel ramdisk:

```bash
python build_initrd.py      # → initrd.tar (also copied to kernel/SkyOS/)
make initrd
```

## Library Support

There is no C libc. Programs `extern crate libsarga;` and use its modules:

- POSIX syscall wrappers (`posix.rs`)
- Memory management / allocator (`mem.rs`)
- Networking and socketpair IPC (`net.rs`)
- GUI windows (`gui.rs`), signals (`signal.rs`), hashing (`hash.rs`), audio (`libskyaudio`), and
  more

`libskyaudio` provides audio playback for programs that need it.
