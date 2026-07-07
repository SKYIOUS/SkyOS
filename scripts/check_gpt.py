"""Verify GPT CRC and partition entries."""
import struct, zlib

with open('release/skyos-0.6.0.iso', 'rb') as f:
    gpt = f.read(92)
    f.seek(1024)
    entries = f.read(16384)
    efi_guid = bytes.fromhex('28732AC11FF8D211BA4B00A0C93EC93B')
    esp_found = False
    for i in range(128):
        pe = entries[i*128:i*128+128]
        if pe[0:16] == efi_guid:
            start = struct.unpack_from('<Q', pe, 32)[0]
            end = struct.unpack_from('<Q', pe, 40)[0]
            print(f'Part {i}: ESP LBA {start}-{end} ({(end-start+1)*512/1024/1024:.1f}MB)')
            esp_found = True
        else:
            start = struct.unpack_from('<Q', pe, 32)[0]
            if start != 0:
                print(f'Part {i}: garbage detected! LBA={start}')

    if not esp_found:
        print('ERROR: ESP partition not found!')
    
    non_zero = sum(1 for i in range(1,128) if entries[i*128:i*128+16] != bytes(16))
    print(f'Non-empty entries besides ESP: {non_zero}')
    
    # CRC check
    crc_raw = struct.unpack('<I', gpt[16:20])[0]
    hdr_zero = bytearray(gpt)
    hdr_zero[16:20] = b'\x00\x00\x00\x00'
    crc_calc = zlib.crc32(hdr_zero) & 0xFFFFFFFF
    print(f'GPT header CRC: stored={crc_raw:#010x} calc={crc_calc:#010x} {"OK" if crc_raw==crc_calc else "MISMATCH"}')
    
    entries_crc_raw = struct.unpack('<I', gpt[88:92])[0]
    entries_crc_calc = zlib.crc32(entries) & 0xFFFFFFFF
    print(f'Entries CRC: stored={entries_crc_raw:#010x} calc={entries_crc_calc:#010x} {"OK" if entries_crc_raw==entries_crc_calc else "MISMATCH"}')
    
    # Check data at start of partition (LBA 136)
    f.seek(136*512)
    data = f.read(32)
    print(f'ESP data at LBA 136: {" ".join(f"{b:02x}" for b in data)}')
    is_fat = data[0] in (0xEB, 0xE9) and data[30:32] == b'\x55\xaa'  # Wrong offset for AA55
    f.seek(136*512+510)
    sig = f.read(2)
    print(f'FAT16: {"YES" if data[0] in (0xEB,0xE9) and sig==b"\\x55\\xaa" else "NO"}')
