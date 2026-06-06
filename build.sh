#!/bin/bash
set -e

echo "Building SkyOS components..."

if [ "$1" == "all" ] || [ -z "$1" ]; then
    cargo build --target x86_64-sarga --release
    echo "Build complete."
    ./disk/create_disk.sh
else
    cargo build --manifest-path $1/Cargo.toml --target x86_64-sarga --release
fi
