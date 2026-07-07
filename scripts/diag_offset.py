"""Check the data at key offsets in the ISO."""
import struct, sys

with open('release/skyos-0.6.0.iso', 'rb') as f:
    data = f.read()

    # MBR partition 1 info
    mbr = data[:512]
    p1_start_lba = struct.unpack('<I', mbr[446+16+8:446+16+12])[0]
    p1_type = mbr[446+16+4]
    print(f'MBR Part 1: type=0x{p1_type:02x} start_LBA={p1_start_lba}')
    print(f'Partition byte offset: {p1_start_lba * 512}')

    # boot entry RBA=16419 (2048-byte LBN)
    rba = 16419
    print(f'El Torito RBA: {rba} (2048-byte LBN)')
    print(f'RBA byte offset: {rba * 2048}')

    # Check what's at the RBA offset
    off_rba = rba * 2048
    off_part = p1_start_lba * 512
    diff = off_part - off_rba
    print(f'Difference: {diff} bytes')

    # Dump data at both offsets
    for off, label in [(off_rba, 'RBA'), (off_part, 'Partition')]:
        chunk = data[off:off+64]
        print(f'{label} byte {off}: {" ".join(f"{b:02x}" for b in chunk[:32])}')
        is_fat = chunk[0] in (0xEB, 0xE9) and data[off+510:off+512] == b'\x55\xaa'
        print(f'  FAT16: {is_fat}')

    # The RBA is 512 bytes before partition start. What's at RBA?
    chunk_rba = data[off_rba:off_rba+512]
    print(f'\nRBA sector:')
    for i in range(0, 512, 16):
        print(f'  {off_rba+i:08x}: {" ".join(f"{b:02x}" for b in chunk_rba[i:i+16])}')

    # Check if there is any important data in the last sector before partition
    # RBA should correspond to the first sector of the ESP data
    # But the partition starts at LBA 65676, which is 512 bytes after the RBA
    # Look at what's at the sector boundary
    sec_before = data[off_rba-512:off_rba]
    print(f'\nSector before RBA:')
    for i in range(0, 512, 16):
        print(f'  {off_rba-512+i:08x}: {" ".join(f"{b:02x}" for b in sec_before[i:i+16])}')

    # Also check: is the appended partition aligned to 2048-byte boundary?
    print(f'\nPartition offset mod 2048: {off_part % 2048}')
    print(f'RBA offset mod 2048: {off_rba % 2048}')

    # README.TXT location
    # LBN 16418, size 42
    rdata = data[16418*2048:16418*2048+2048]
    print(f'\nREADME.TXT content: {rdata[:42]}')
