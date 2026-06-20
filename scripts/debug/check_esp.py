"""Check ESP contents for bootloader."""
import struct

with open(r'C:\Users\nanda\Desktop\Github\SkyOS\build\esp.img', 'rb') as f:
    esp = f.read()

sec_per_cluster = 4
bytes_per_sec = 512
cluster_size = sec_per_cluster * bytes_per_sec

data_start_sector = 1 + (2 * 64) + (512 * 32 // 512)
data_start = data_start_sector * 512

cluster_offset = data_start + (2 - 2) * cluster_size

print('=== EFI Directory ===')
for i in range(0, cluster_size, 32):
    entry = esp[cluster_offset + i : cluster_offset + i + 32]
    name = entry[0:11]
    attr = entry[11]
    if name[0:1] == b'\x00':
        break
    if name[0:1] == b'\xE5':
        continue
    if attr == 0x0F:
        continue
    name_str = name.decode('ascii', errors='replace').strip()
    first_cluster = struct.unpack_from('<H', entry, 26)[0]
    file_size = struct.unpack_from('<I', entry, 28)[0]
    if file_size > 0 or attr == 0x10:
        print(f'  {name_str:15s} attr={attr:02x} cluster={first_cluster} size={file_size}')

# Search for BOOT in the ESP
print('\n=== Searching for BOOT ===')
idx = esp.find(b'BOOT')
while idx >= 0:
    name = esp[idx:idx+20]
    print(f'  offset {idx}: {name}')
    idx = esp.find(b'BOOT', idx + 1)

# Also check if there's a BOOT directory under EFI
# EFI dir cluster = 2, let's check subdirs
# BOOT subdirectory should be inside EFI
for i in range(0, cluster_size, 32):
    entry = esp[cluster_offset + i : cluster_offset + i + 32]
    name = entry[0:11]
    attr = entry[11]
    if attr == 0x10 and name[0:1] not in (b'\x00', b'\xE5') and name.strip() not in (b'.', b'..'):
        name_str = name.decode('ascii', errors='replace').strip()
        cluster = struct.unpack_from('<H', entry, 26)[0]
        if cluster > 1:
            sub_offset = data_start + (cluster - 2) * cluster_size
            print(f'\n=== {name_str} subdirectory (cluster {cluster}) ===')
            for j in range(0, cluster_size, 32):
                sub_entry = esp[sub_offset + j : sub_offset + j + 32]
                sub_name = sub_entry[0:11]
                sub_attr = sub_entry[11]
                if sub_name[0:1] == b'\x00':
                    break
                if sub_name[0:1] == b'\xE5':
                    continue
                if sub_attr == 0x0F:
                    continue
                sn = sub_name.decode('ascii', errors='replace').strip()
                sc = struct.unpack_from('<H', sub_entry, 26)[0]
                sz = struct.unpack_from('<I', sub_entry, 28)[0]
                print(f'    {sn:15s} cluster={sc} size={sz}')
