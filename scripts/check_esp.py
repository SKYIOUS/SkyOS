"""Check ESP FAT BPB."""
import struct

path = "C:\\Users\\nanda\\Desktop\\Github\\SKYIOUS KERNEL\\target\\x86_64-vahi\\debug\\bootimage-vahi_kernel.bin"
with open(path, "rb") as f:
    d = f.read()

esp = d[17408:17408+512]
print("ESP first 64 bytes:")
for i in range(0, 64, 16):
    h = " ".join(f"{b:02x}" for b in esp[i:i+16])
    print(f"  [{i:3d}] {h}")

print()
print(f"Bytes per sector:     {struct.unpack('<H', esp[11:13])[0]}")
print(f"Sectors per cluster:  {esp[13]}")
print(f"Reserved sectors:     {struct.unpack('<H', esp[14:16])[0]}")
print(f"Number of FATs:       {esp[16]}")
print(f"Root entries (FAT16): {struct.unpack('<H', esp[17:19])[0]}")
print(f"Total sectors (16):   {struct.unpack('<H', esp[19:21])[0]}")
print(f"Media descriptor:     {esp[21]:#x}")
print(f"FAT size (16):        {struct.unpack('<H', esp[22:24])[0]}")

# FAT32 extended fields
fat32_fat = struct.unpack('<I', esp[36:40])[0]
fat32_root = struct.unpack('<I', esp[44:48])[0]
fat32_total = struct.unpack('<I', esp[32:36])[0]
fs_ver = struct.unpack('<H', esp[42:44])[0]
print(f"FAT32: FAT sectors    {fat32_fat}")
print(f"FAT32: Root cluster   {fat32_root}")
print(f"FAT32: Total sectors  {fat32_total}")
print(f"FAT32: FS version     {fs_ver}")
print(f"Boot sig: {esp[510]:#x} {esp[511]:#x}")
print(f"OEM: {esp[3:11]}")

# This is FAT16.
bps = struct.unpack('<H', esp[11:13])[0]
spc = esp[13]
reserved = struct.unpack('<H', esp[14:16])[0]
num_fats = esp[16]
root_entries = struct.unpack('<H', esp[17:19])[0]
fat_size = struct.unpack('<H', esp[22:24])[0]

root_dir_sectors = (root_entries * 32 + bps - 1) // bps
data_start = (reserved + num_fats * fat_size + root_dir_sectors) * bps
root_start = (reserved + num_fats * fat_size) * bps  # absolute byte offset in ESP

print(f"FAT16: reserved={reserved} fats={num_fats} fat_secs={fat_size}")
print(f"FAT16: root_entries={root_entries} root_dir_secs={root_dir_sectors}")
print(f"Root dir byte offset in ESP: {root_start}")
print(f"Data region byte offset in ESP: {data_start}")

# Scan root dir
esp_all = d[17408:17408+16776704]
print()
print("Root directory entries:")
for off in range(root_start, min(root_start + root_dir_sectors * bps, len(esp_all)), 32):
    entry = esp_all[off:off+32]
    if entry[0] == 0: break
    if entry[0] == 0xE5: continue
    attr = entry[11]
    if attr & 0x0F == 0x0F: continue
    name = entry[0:11].decode("latin-1", errors="replace")
    is_dir = "DIR" if attr & 0x10 else "FILE"
    cluster = struct.unpack('<H', entry[26:28])[0]  # FAT16: low 16 bits only
    size = struct.unpack('<I', entry[28:32])[0]
    if name.strip():
        print(f"  {is_dir} '{name}' cluster={cluster} size={size}")
    if name.strip().upper() == "EFI":
        # Follow subdirectory
        sub_cluster = cluster
        sub_start = data_start + (sub_cluster - 2) * spc * bps
        print(f"    Subdir at offset {sub_start}:")
        for sub_off in range(sub_start, min(sub_start + spc * bps, len(esp_all)), 32):
            sub_entry = esp_all[sub_off:sub_off+32]
            if sub_entry[0] == 0: break
            if sub_entry[0] == 0xE5: continue
            sub_attr = sub_entry[11]
            if sub_attr & 0x0F == 0x0F: continue
            sub_name = sub_entry[0:11].decode("latin-1", errors="replace")
            sub_clust = struct.unpack('<H', sub_entry[26:28])[0]
            sub_sz = struct.unpack('<I', sub_entry[28:32])[0]
            sd = "DIR" if sub_attr & 0x10 else "FILE"
            if sub_name.strip():
                print(f"    {sd} '{sub_name}' cluster={sub_clust} size={sub_sz}")
            # Follow BOOT subdir (note: FAT short name is uppercase, padded)
            if sub_name.strip().upper().startswith("BOOT"):
                boot_cluster = sub_clust
                boot_start = data_start + (boot_cluster - 2) * spc * bps
                print(f"      Subdir at offset {boot_start}:")
                for b_off in range(boot_start, min(boot_start + spc * bps, len(esp_all)), 32):
                    b_entry = esp_all[b_off:b_off+32]
                    if b_entry[0] == 0: break
                    if b_entry[0] == 0xE5: continue
                    b_attr = b_entry[11]
                    if b_attr & 0x0F == 0x0F: continue
                    b_name = b_entry[0:11].decode("latin-1", errors="replace")
                    b_clust = struct.unpack('<H', b_entry[26:28])[0]
                    b_sz = struct.unpack('<I', b_entry[28:32])[0]
                    bd = "DIR" if b_attr & 0x10 else "FILE"
                    if b_name.strip():
                        print(f"      {bd} '{b_name}' cluster={b_clust} size={b_sz}")
