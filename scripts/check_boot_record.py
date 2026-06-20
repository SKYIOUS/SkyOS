"""Verify boot record structure."""
import struct

with open(r'C:\Users\nanda\Desktop\Github\SkyOS\build\test.iso', 'rb') as f:
    f.seek(17 * 2048)
    br = f.read(2048)

print('Boot Record:')
print(f'  Type: {br[0]:02x}')
print(f'  ID: {br[1:6]}')
print(f'  Version: {br[6]:02x}')
sys_id = br[7:39].decode('ascii', errors='replace').rstrip(chr(0))
print(f'  System ID: [{sys_id}]')

lba_le = struct.unpack('<I', br[39:43])[0]
lba_be = struct.unpack('>I', br[43:47])[0]
print(f'  Boot Catalog LBA (LE): {lba_le}')
print(f'  Boot Catalog LBA (BE): {lba_be}')

# Check boot system use (should be all zeros for compliance)
bsu = br[71:2048]
nonzero = sum(1 for b in bsu if b != 0)
print(f'  Boot system use ({len(bsu)} bytes): {nonzero} non-zero bytes')
for i, b in enumerate(bsu):
    if b:
        print(f'    Non-zero at offset {i}: {b:02x}')

# Also check if the El Torito specification string is in the right place
print(f'  Expected \"EL TORITO SPECIFICATION\": {br[7:39]!r}')
