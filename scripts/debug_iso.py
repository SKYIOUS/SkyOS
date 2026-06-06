"""Detailed ISO debug check."""
import struct

path = "C:\\Users\\nanda\\Desktop\\Github\\SkyOS\\skyos-installer.iso"
with open(path, "rb") as f:
    data = f.read()

# Boot catalog is at sector 19
bc = data[19*2048:20*2048]
print("Boot catalog sector 19 first 64 bytes:")
for i in range(0, 64, 16):
    hexs = ' '.join(f'{b:02x}' for b in bc[i:i+16])
    print(f"  [{i:3d}] {hexs}")

ie = bc[32:64]
sect_count = struct.unpack("<H", ie[6:8])[0]
load_rba = struct.unpack("<I", ie[8:12])[0]
print(f"\nSector count: {sect_count}")
print(f"Load RBA: {load_rba}")

# Check boot image at load_rba
boot_off = load_rba * 2048
print(f"\nBoot image at offset {boot_off} (sector {load_rba})")
print(f"First 32 bytes: {data[boot_off:boot_off+32].hex()}")

# Check boot image contents
# First 512 bytes should be MBR (GPT protective)
import struct
mbr_sig = struct.unpack("<H", data[boot_off+510:boot_off+512])[0]
print(f"MBR signature at boot offset+510: 0x{mbr_sig:04X}")

# Check for GPT header at boot offset + 512
if data[boot_off+512:boot_off+520] == b'EFI PART':
    print("GPT header found at boot offset+512")
else:
    print("No GPT header at boot offset+512")

# Check second boot entry in catalog (bytes 64-95)
bc = data[19*2048:20*2048]
entry2_rba = struct.unpack("<I", bc[72:76])[0]
entry2_count = struct.unpack("<H", bc[70:72])[0]
print(f"\nSecond entry (BIOS): rba={entry2_rba} count={entry2_count}")
