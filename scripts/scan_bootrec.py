"""Scan for Boot Record and check catalog pointer."""
import struct

with open('release/skyos-0.6.0.iso', 'rb') as f:
    data = f.read()

# Scan volume descriptors (LBAs 16+)
for lba in range(16, 100):
    vd = data[lba * 2048:lba * 2048 + 7]
    if vd[0:1] == b'\x00':  # Boot Record
        sys_id = data[lba*2048+7:lba*2048+39]
        cat_ptr71 = struct.unpack_from('<I', data[lba*2048:], 71)[0]
        cat_ptr72 = struct.unpack_from('<I', data[lba*2048:], 72)[0]
        print(f'LBA {lba}: Boot Record: sys_id={sys_id}')
        print(f'  catalog_ptr off71: {cat_ptr71}')
        print(f'  catalog_ptr off72: {cat_ptr72}')
    elif vd[0:1] == b'\x01':  # PVD
        print(f'LBA {lba}: PVD')
    elif vd[0:1] == b'\xff':  # Terminator
        print(f'LBA {lba}: Volume Descriptor Set Terminator')
        break
    else:
        print(f'LBA {lba}: type={vd[0]} id={vd[1:6]}')

# Boot Record at LBA 17: hex dump around the catalog pointer field
br = data[17*2048:18*2048]
print(f'\nBytes 64-80 of Boot Record at LBA 17:')
for i in range(64, 80, 2):
    val = struct.unpack_from('<H', br, i)[0]
    print(f'  offset {i}: uint16 = {val} (0x{val:04x})')

print(f'\nFull hex of Boot Record:')
for i in range(0, 2048, 16):
    hex_str = ' '.join(f'{b:02x}' for b in br[i:i+16])
    print(f'  {i:4d}: {hex_str}')
