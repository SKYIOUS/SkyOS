#!/bin/sh
# Self-hosting build script for SkyOS
# Run on SkyOS to build the entire operating system from source.
#
# Build order follows dependency chain:
#   1. libsarga      — system library (core deps)
#   2. coreutils     — basic userland utilities
#   3. sash          — shell
#   4. init          — init process
#   5. svc           — service manager
#   6. skypkg        — package manager (needed for repo)
#   7. skybuild      — build tool (self-referential)
#   8. terminal, mixer, clock — GUI applications
#   9. repo          — generate local package repository

set -e

REPO_DIR="${1:-/repo}"
echo "=== SkyOS Self-Hosting Build ==="
echo "Repository: $REPO_DIR"
echo ""

build_pkg() {
    local name="$1"
    local recipe="${name}.recipe"
    if [ ! -f "$recipe" ]; then
        echo "WARNING: Recipe $recipe not found, skipping $name"
        return 1
    fi
    echo "--- Building $name ---"
    skybuild build "$recipe" || {
        echo "FAILED: $name"
        exit 1
    }
    echo "--- Installed $name ---"
}

install_pkg() {
    local name="$1"
    local skp="${name}.skp"
    if [ -f "$skp" ]; then
        echo "--- Installing $name ---"
        skybuild install "$skp" || echo "WARNING: install failed for $name"
    fi
}

# Phase 1: Core libraries
build_pkg libsarga
install_pkg libsarga

# Phase 2: Base userland
build_pkg coreutils
install_pkg coreutils

# Phase 3: Shell
build_pkg sash
install_pkg sash

# Phase 4: System services
build_pkg init
install_pkg init

build_pkg svc
install_pkg svc

# Phase 5: Package management
build_pkg skypkg
install_pkg skypkg

# Phase 6: Build tool
build_pkg skybuild
install_pkg skybuild

# Phase 7: GUI applications
build_pkg terminal
install_pkg terminal

build_pkg mixer
install_pkg mixer

build_pkg clock
install_pkg clock

# Phase 8: Generate repository index
echo "=== Generating Repository ==="
mkdir -p "$REPO_DIR"
cp -f *.skp "$REPO_DIR/" 2>/dev/null || true
skybuild repo "$REPO_DIR"

echo ""
echo "=== Build Complete ==="
echo "Packages are in: $REPO_DIR"
echo "To install from repo: skypkg install <name>"
