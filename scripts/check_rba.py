"""Check FAT16 at different RBA offsets."""
import struct

with open('release/skyos-0.6.0.iso', 'rb') as f:
    data = f.read()
    total = len(data)
    print(f'Total ISO size: {total} bytes ({total/1024/1024:.1f} MB)')

    rba = 16419
    off_512 = rba * 512
    off_2048 = rba * 2048
    chunk_512 = data[off_512:off_512+512]
    chunk_2048 = data[off_2048:off_2048+512]
    is_fat_512 = chunk_512[0] in (0xEB, 0xE9) and data[off_512+510:off_512+512] == b'\x55\xaa'
    is_fat_2048 = chunk_2048[0] in (0xEB, 0xE9) and data[off_2048+510:off_2048+512] == b'\x55\xaa'
    print(f'RBA={rba} as 512-byte (byte {off_512}): FAT16={"YES" if is_fat_512 else "NO"}')
    print(f'RBA={rba} as 2048-byte (byte {off_2048}): FAT16={"YES" if is_fat_2048 else "NO"}')
    if is_fat_512:
        oem = chunk_512[3:11].decode('ascii', errors='replace')
        print(f'  OEM: {oem}')
    if is_fat_2048:
        oem = chunk_2048[3:11].decode('ascii', errors='replace')
        print(f'  OEM: {oem}')

    # Search for all FAT16 volumes (OEM 'MSWIN4.1')
    off = data.find(b'MSWIN4.1')
    while off != -1:
        print(f'FAT16 at byte {off} (LBN {off/2048:.1f}, LBA {off/512:.0f})')
        off = data.find(b'MSWIN4.1', off + 1)
