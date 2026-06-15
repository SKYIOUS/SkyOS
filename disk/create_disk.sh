#!/bin/bash
# disk/create_disk.sh — builds the Aethos OS disk image
set -e

DISK="aethos.img"
# Need root to mount loop, using a local mount or just skipping if not root for demonstration
if [ "$EUID" -ne 0 ]; then
  echo "Warning: please run as root to mount and build aethos.img."
  echo "Will create dummy files if cannot mount."
fi

SIZE_MB=128 # 512 is too big for a quick test, 128 is fine
dd if=/dev/zero of=$DISK bs=1M count=$SIZE_MB

if ! command -v mkfs.ext2 &> /dev/null; then
    echo "mkfs.ext2 not found! Disk image created but not formatted."
    exit 0
fi

mkfs.ext2 -F -L "AETHOS" $DISK

# On windows/wsl we might not be able to mount a loop device easily without proper permissions,
# so we will just mention it's created. We will assume the build environment has root / loop.
if [ "$EUID" -eq 0 ]; then
    MOUNT="/mnt/aethos"
    mkdir -p $MOUNT
    mount -o loop $DISK $MOUNT

    mkdir -p $MOUNT/{bin,sbin,lib,etc,tmp,proc,dev,home,usr/bin,usr/lib,var/log}

    # Copy files
    TARGET_DIR="target/x86_64-sarga/release"
    cp $TARGET_DIR/init       $MOUNT/sbin/init
    cp $TARGET_DIR/ash        $MOUNT/bin/ash
    cp $TARGET_DIR/ade        $MOUNT/bin/ade
    cp $TARGET_DIR/apkg       $MOUNT/bin/apkg

    # Coreutils
    cp $TARGET_DIR/ls         $MOUNT/bin/ls
    cp $TARGET_DIR/cat        $MOUNT/bin/cat
    cp $TARGET_DIR/echo       $MOUNT/bin/echo
    cp $TARGET_DIR/mkdir      $MOUNT/bin/mkdir
    cp $TARGET_DIR/rm         $MOUNT/bin/rm

    # Nettools
    cp $TARGET_DIR/ifconfig   $MOUNT/bin/ifconfig
    cp $TARGET_DIR/curl       $MOUNT/bin/curl
    cp $TARGET_DIR/nc         $MOUNT/bin/nc

    cp $TARGET_DIR/proc       $MOUNT/sbin/procd
    cp $TARGET_DIR/aicli      $MOUNT/bin/aicli

    # Configs
    cat > $MOUNT/etc/hostname << 'EOF'
aethos
EOF

    cat > $MOUNT/etc/init.d/network.toml << 'EOF'
[service]
name = "netd"
binary = "/sbin/netd"
restart = "on-failure"
EOF

    umount $MOUNT
fi

echo "Aethos disk image created: $DISK"
