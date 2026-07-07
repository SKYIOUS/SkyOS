"""Parse ISO root directory for all files."""
import struct, sys

with open(sys.argv[1], 'rb') as f:
    data = f.read()
    pvd = data[16*2048:17*2048]
    root_loc = struct.unpack_from('<I', pvd, 158)[0]
    root_sz = struct.unpack_from('<I', pvd, 166)[0]
    rd = data[root_loc*2048:root_loc*2048+root_sz]
    print(f'Root dir at LBN {root_loc}, size {root_sz}')
    off = 0
    while off < len(rd)-32:
        rec_len = rd[off]
        if rec_len == 0:
            break
        ext_loc = struct.unpack_from('<I', rd, off+2)[0]
        ext_sz = struct.unpack_from('<I', rd, off+10)[0]
        if off+32 < len(rd):
            name_len = rd[off+32]
            name = rd[off+33:off+33+name_len].decode('ascii', errors='replace').split(';')[0].rstrip('.')
            flags = rd[off+25]
            is_dir = bool(flags & 2)
            tag = '(dir)' if is_dir else '     '
            print(f'  {tag} {name:20s} LBN={ext_loc} size={ext_sz}')
        off += rec_len

    # Dump Boot Record at LBN 17
    br = data[17*2048:17*2048+72]
    cat_lbn = struct.unpack_from('<I', br, 39)[0]
    print(f'\nBoot Record: type={br[0]} sys_id={br[7:39]} cat_lbn={cat_lbn}')

    # Even if cat_lbn is 0, try to find boot catalog in the data
    # Search for validation entry signature: first byte = 0x01, second byte = platform
    # Search in LBN 17-50
    for lbn in range(17, 50):
        off = lbn * 2048
        sector = data[off:off+2048]
        if sector[0:2] == b'\x01\xef':  # Validation entry with UEFI platform
            print(f'Found candidate boot catalog at LBN {lbn}')
            # Read validation + boot entry
            val = sector[:32]
            boot = sector[32:64]
            total = sum(int.from_bytes(val[i:i+2], 'big') for i in range(0,32,2))
            chk = int.from_bytes(val[28:30], 'big')
            ind = boot[0]
            load_rba = struct.unpack_from('<I', boot, 8)[0]
            print(f'  Platform: {val[1]:#04x} Checksum: {chk:#06x} ({"OK" if total%65536==0 else "BAD"})')
            print(f'  Indicator: {ind:#04x} LoadRBA: {load_rba}')
