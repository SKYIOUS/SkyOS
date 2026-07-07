"""Diagnose data at key LBAs in the ISO."""
import struct, sys

with open(sys.argv[1], 'rb') as f:
    data = f.read()
    
    # What's at 2048-byte LBN 16419 (appended partition)?
    off = 16419 * 2048
    bd = data[off:off+512]
    is_fat = bd[0] in (0xEB, 0xE9) and bd[510:512] == b'\x55\xaa'
    print(f'LBN 16419 (appended partition, byte {off}):')
    print(f'  First 48: {" ".join(f"{b:02x}" for b in bd[:48])}')
    print(f'  FAT16: {"YES" if is_fat else "NO"}')
    if is_fat:
        oem = bd[3:11].decode('ascii', errors='replace')
        bps = struct.unpack('<H', bd[11:13])[0]
        spc = bd[13]
        print(f'  OEM={oem} BpS={bps} SpC={spc}')
    
    # What's at LBN 34 (ESP.IMG file)?
    off2 = 34 * 2048
    bd2 = data[off2:off2+512]
    is_fat2 = bd2[0] in (0xEB, 0xE9) and bd2[510:512] == b'\x55\xaa'
    print(f'LBN 34 (ESP.IMG file, byte {off2}):')
    print(f'  First 48: {" ".join(f"{b:02x}" for b in bd2[:48])}')
    print(f'  FAT16: {"YES" if is_fat2 else "NO"}')
    if is_fat2:
        oem = bd2[3:11].decode('ascii', errors='replace')
        bps = struct.unpack('<H', bd2[11:13])[0]
        spc = bd2[13]
        print(f'  OEM={oem} BpS={bps} SpC={spc}')
    
    # Are they the same?
    print(f'\nLBN 34 and LBN 16419 are SAME: {bd[:32] == bd2[:32]}')
    
    # Check the MBR partition 1 start
    mbr = data[:512]
    p1_start = struct.unpack('<I', mbr[446+16+8:446+16+12])[0]
    p1_type = mbr[446+16+4]
    p1_size = struct.unpack('<I', mbr[446+16+12:446+16+16])[0]
    print(f'\nMBR Partition 1: type={p1_type:#04x} start={p1_start} size={p1_size}')
    
    # What's at MBR partition 1 start?
    off3 = p1_start * 512
    bd3 = data[off3:off3+512]
    is_fat3 = bd3[0] in (0xEB, 0xE9) and bd3[510:512] == b'\x55\xaa'
    print(f'MBR Part1 data (byte {off3}): FAT16: {"YES" if is_fat3 else "NO"}')
    print(f'  First 48: {" ".join(f"{b:02x}" for b in bd3[:48])}')
    
    # Check: are LBN 16419 and MBR Part1 the same?
    print(f'LBN 16419 == MBR Part1: {bd[:32] == bd3[:32]}')
    
    # Root dir at LBN 19 - find ESP.IMG extent
    rd = data[19*2048:19*2048+2048]
    off = 0
    while off < len(rd)-32:
        rl = rd[off]
        if rl == 0: break
        ext = struct.unpack_from('<I', rd, off+2)[0]
        sz = struct.unpack_from('<I', rd, off+10)[0]
        nl = rd[off+32]
        nm = rd[off+33:off+33+nl].decode('ascii', errors='replace').split(';')[0].rstrip('.')
        print(f'\nFile: {nm} at LBN={ext} size={sz}')
        if nm.upper() == 'ESP.IMG':
            # Dump its actual content
            fd = data[ext*2048:ext*2048+512]
            print(f'  FAT16: {"YES" if fd[0] in (0xEB,0xE9) and fd[510:512]==b"\\x55\\xaa" else "NO"}')
        off += rl
