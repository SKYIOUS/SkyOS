import struct, sys
sys.path.insert(0, 'C:\\Users\\nanda\\Desktop\\Github\\SkyOS\\scripts')
bootimg_path = 'C:\\Users\\nanda\\Desktop\\Github\\SKYIOUS KERNEL\\target\\x86_64-vahi\\debug\\bootimage-vahi_kernel.bin'
with open(bootimg_path, 'rb') as f:
    bootimg = f.read()

ESP_GUID = bytes([0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b])
part_lba = struct.unpack('<Q', bootimg[584:592])[0]
part_count = struct.unpack('<I', bootimg[592:596])[0]
for i in range(min(part_count, 128)):
    off = (part_lba + i) * 512
    entry = bootimg[off:off+128]
    ptype = entry[0:16]
    if ptype == ESP_GUID:
        start_lba = struct.unpack('<Q', entry[32:40])[0]
        end_lba = struct.unpack('<Q', entry[40:48])[0]
        esp_start = start_lba * 512
        esp_size = (end_lba - start_lba + 1) * 512
        esp_data = bootimg[esp_start:esp_start + esp_size]
        print(f'ESP: {esp_start}-{esp_start+esp_size} ({esp_size} bytes)')
        break

from build_installer_iso import extract_fat_file, build_fat32_image
efi = extract_fat_file(esp_data, 'EFI/BOOT/BOOTX64.EFI')
kernel = extract_fat_file(esp_data, 'vahi_kernel.bin')
if kernel is None:
    kernel = extract_fat_file(esp_data, 'KERNEL~1')
print(f'BOOTX64.EFI: {len(efi) if efi else 0} bytes')
print(f'vahi_kernel.bin: {len(kernel) if kernel else 0} bytes')

fat32 = build_fat32_image([
    ('EFI/BOOT/BOOTX64.EFI', efi),
    ('vahi_kernel.bin', kernel),
])
print(f'FAT32 image: {len(fat32)} bytes')
print(f'Bytes/sector: {struct.unpack("<H", fat32[11:13])[0]}')
print(f'Sectors/cluster: {fat32[13]}')
print(f'Reserved: {struct.unpack("<H", fat32[14:16])[0]}')
print(f'Total sectors: {struct.unpack("<I", fat32[32:36])[0]}')
print(f'FAT size: {struct.unpack("<I", fat32[64:68])[0]}')
print(f'Root cluster: {struct.unpack("<I", fat32[72:76])[0]}')
print(f'Boot sig: {fat32[510]:02x}{fat32[511]:02x}')
