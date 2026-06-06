#!/bin/bash
set -e

echo "Ensuring target x86_64-unknown-none is installed..."
rustup target add x86_64-unknown-none

echo "Building Aethos OS components..."

# Loop over each member in the workspace and build for the aethos target if we had one
# For now, we compile for x86_64-unknown-none or the native target. We'll use the default target
# for the build process, simulating a standalone OS environment. Let's assume a custom target JSON
# or we just build normal static binaries using x86_64-unknown-none for no_std.

# Actually, user space executables might need a target. Assuming we compile for standard x86_64-unknown-linux-musl
# or a custom x86_64-aethos target (as mentioned in the prompt). Since we don't have a rust custom target yet,
# we will just do `cargo build --release` which assumes standard std environment,
# OR we do `cargo build --release --target x86_64-unknown-none`. The prompt mentions `target/x86_64-aethos/release/...`.
# We will create a custom target file `x86_64-aethos.json`.

if [ "$1" == "all" ] || [ -z "$1" ]; then
    cargo build --target x86_64-unknown-none --release
    echo "Build complete."
    ./disk/create_disk.sh
else
    cargo build --manifest-path $1/Cargo.toml --target x86_64-unknown-none --release
fi
