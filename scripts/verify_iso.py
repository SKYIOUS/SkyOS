"""Verify ISO boot catalog."""
import struct
path = "C:\\Users\\nanda\\Desktop\\Github\\SkyOS\\skyos-installer.iso"
with open(path, "rb") as f:
    d = f.read()

bvd = d[17*2048:18*2048]
cat_lba = struct.unpack("<I", bvd[9:13])[0]
print(f"BVD cat_lba = {cat_lba}")

bc = d[cat_lba*2048:(cat_lba+1)*2048]
print(f"BootCat header={bc[0]} platform={bc[1]}")
iec = struct.unpack("<H", bc[38:40])[0]
ier = struct.unpack("<I", bc[40:44])[0]
print(f"InitEntry: bootable={bc[32]:02x} media={bc[33]} count={iec} rba={ier}")

# Second entry
iec2 = struct.unpack("<H", bc[70:72])[0]
ier2 = struct.unpack("<I", bc[72:76])[0]
print(f"Entry 2: bootable={bc[64]:02x} media={bc[65]} count={iec2} rba={ier2}")

boot_off = ier * 2048
print(f"Boot image offset: {boot_off} ({ier} sectors)")
print(f"First 4 bytes: {d[boot_off:boot_off+4].hex()}")
print(f"MBR sig: {d[boot_off+510]:02x}{d[boot_off+511]:02x}")
