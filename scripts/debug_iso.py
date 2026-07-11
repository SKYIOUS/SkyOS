"""Debug standard ISO structure."""
import struct, sys

path = sys.argv[1] if len(sys.argv) > 1 else 'release/skyos-0.6.0.iso'

with open(path, 'rb') as f:
    data = f.read()

total = len(data)
print(f'File: {total:,} bytes ({total//2048} sectors of 2048)')

# Find catalog from root directory
root_loc = struct.unpack_from('<I', data[16*2048:17*2048], 158)[0]
rd = data[root_loc * 2048:root_loc * 2048 + 4096]
cat_lbn = None
off = 0
while off < len(rd) - 32:
    rec_len = rd[off]
    if rec_len == 0: break
    name_len = rd[off + 32]
    name = rd[off + 33:off + 33 + name_len].decode('ascii', errors='replace').split(';')[0].rstrip('.')
    flba = struct.unpack_from('<I', rd, off + 2)[0]
    fsz = struct.unpack_from('<I', rd, off + 10)[0]
    print(f'  {name}: LBA={flba} size={fsz:,}')
    if 'CATALOG' in name.upper(): cat_lbn = flba
    off += rec_len

# Catalog
cat = data[cat_lbn * 2048:cat_lbn * 2048 + 2048]
val = cat[0:32]
init = cat[32:64]
print(f'\nValidation entry: {val.hex()}')
print(f'Init entry: {init.hex()}')
print(f'RBA at off8 = {struct.unpack_from("<I", init, 8)[0]}')
print(f'RBA at off16 = {struct.unpack_from("<I", init, 16)[0]}')
print(f'Sec count = {struct.unpack_from("<H", init, 6)[0]}')
print(f'Platform = {val[0]}')
s = sum(val) & 0xFFFF
print(f'Validation sum = {s:#06x} valid={s==0}')

# Check what's at RBA 35
for rba in [struct.unpack_from("<I", init, 8)[0], struct.unpack_from("<I", init, 16)[0], 35]:
    if rba <= 0 or rba * 2048 >= total: continue
    chunk = data[rba * 2048:rba * 2048 + 512]
    ok = chunk[510:512] == b'\x55\xAA'
    oem = chunk[3:11]
    print(f'\nRBA {rba}: sig=55aa?{ok} OEM={oem} start={chunk[0:3].hex()}')
    if ok:
        bps = struct.unpack_from('<H', chunk, 11)[0]
        print(f'  bytes_per_sector={bps} media={chunk[21]:#04x}')
