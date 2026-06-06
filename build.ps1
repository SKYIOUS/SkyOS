$ErrorActionPreference = "Stop"

Write-Host "Building SkyOS components..."

if ($args.Count -eq 0 -or $args[0] -eq "all") {
    cargo build --target x86_64-sarga --release
    Write-Host "Build complete."
    
    # Attempt to build the disk image using WSL since mkfs.ext2 requires Linux
    Write-Host "Packaging disk image via WSL..."
    wsl ./disk/create_disk.sh
} else {
    $component = $args[0]
    cargo build --manifest-path "$component/Cargo.toml" --target x86_64-sarga --release
}
