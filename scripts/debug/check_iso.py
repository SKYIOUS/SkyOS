"""Verify ISO structure."""
import struct

path = "C:\\Users\\nanda\\Desktop\\Github\\SkyOS\\skyos-installer.iso"
with open(path, "rb") as f:
    data = f.read()

print(f"Total size: {len(data):,} bytes")
print()

# MBR
mbr = data[:512]
sig = struct.unpack("<H", mbr[510:512])[0]
print(f"MBR signature: 0x{sig:04X} (expect 0xAA55)")
pt = mbr[446:510]
for i in range(4):
    e = pt[i*16:(i+1)*16]
    if e[0] or e[4]:
        lba = struct.unpack("<I", e[8:12])[0]
        size = struct.unpack("<I", e[12:16])[0]
        print(f"  Part {i}: boot=0x{e[0]:02X} type=0x{e[4]:02X} lba={lba} size={size}")

# PVD
pvd = data[16*2048:17*2048]
print(f"PVD type={pvd[0]} id={pvd[1:6]} vol_id={pvd[40:72]}")

# BVD
bvd = data[17*2048:18*2048]
cat_lba = struct.unpack("<I", bvd[9:13])[0]
print(f"BVD type={bvd[0]} cat_lba={cat_lba}")

# Boot Catalog
bc = data[cat_lba*2048:(cat_lba+1)*2048]
platforms = {0: "x86 BIOS", 0xEF: "UEFI"}
print(f"BootCat header={bc[0]} platform={platforms.get(bc[1], f'0x{bc[1]:02X}')}")

# Initial Entry
ie = bc[32:64]
rba = struct.unpack("<I", ie[8:12])[0]
count = struct.unpack("<H", ie[6:8])[0]
print(f"InitEntry: bootable=0x{ie[0]:02X} media={ie[1]} count={count} rba={rba}")

# Verify boot image at RBA
boot_off = rba * 2048
boot_size = len(data) - boot_off
print(f"Boot image at sector {rba}, offset {boot_off}, size ~{boot_size:,} bytes")
print(f"First bytes: {data[boot_off:boot_off+8].hex()}")

# isohybrid check
print(f"MBR at byte 0: present")
print(f"ISO+MBR hybrid: {'YES' if sig == 0xAA55 else 'NO'}")
