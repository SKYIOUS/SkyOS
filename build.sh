#!/bin/bash
set -e

echo "Building SARGA OS components..."

if [ "$1" == "all" ] || [ -z "$1" ]; then
    cargo build -Zbuild-std=core,alloc --target x86_64-sarga.json  --release
    echo "Build complete."
    ./disk/create_disk.sh
else
    cargo build -Zbuild-std=core,alloc --manifest-path $1/Cargo.toml --target x86_64-sarga.json  --release
fi
