"""Compare GPT from ISO with GPT from working bootimage."""
import struct, zlib

iso_path = 'release/skyos-0.6.0.iso'
bootimg_path = '../SKYIOUS KERNEL/target/x86_64-vahi/debug/bootimage-vahi_kernel.bin'

for name, path in [('ISO', iso_path), ('Bootimage', bootimg_path)]:
    with open(path, 'rb') as f:
        # Read GPT header at LBA 1
        f.seek(512)
        gpt = bytearray(f.read(92))
        sig = gpt[0:8].decode('ascii', errors='replace')
        rev = struct.unpack('<I', gpt[8:12])[0]
        hdr_sz = struct.unpack('<I', gpt[12:16])[0]
        crc = struct.unpack('<I', gpt[16:20])[0]
        my_lba = struct.unpack('<Q', gpt[24:32])[0]
        alt_lba = struct.unpack('<Q', gpt[32:40])[0]
        first_usable = struct.unpack('<Q', gpt[40:48])[0]
        last_usable = struct.unpack('<Q', gpt[48:56])[0]
        guid = gpt[56:72].hex()
        pe_lba = struct.unpack('<Q', gpt[72:80])[0]
        num_parts = struct.unpack('<I', gpt[80:84])[0]
        part_sz = struct.unpack('<I', gpt[84:88])[0]
        pe_crc = struct.unpack('<I', gpt[88:92])[0]
        print(f'\n=== {name} GPT ===')
        print(f'Signature: {sig}')
        print(f'Revision: {rev:#x}')
        print(f'Header size: {hdr_sz}')
        print(f'CRC: {crc:#010x}')
        print(f'My LBA: {my_lba}')
        print(f'Alt LBA: {alt_lba}')
        print(f'First usable: {first_usable}')
        print(f'Last usable: {last_usable}')
        print(f'Disk GUID: {guid}')
        print(f'Part entries LBA: {pe_lba}')
        print(f'Num entries: {num_parts}')
        print(f'Entry size: {part_sz}')
        print(f'Entries CRC: {pe_crc:#010x}')
        
        # CRC verification
        gpt_for_crc = bytearray(gpt)
        gpt_for_crc[16:20] = b'\x00\x00\x00\x00'
        calc_crc = zlib.crc32(gpt_for_crc) & 0xFFFFFFFF
        print(f'CRC calc: {calc_crc:#010x} {"OK" if crc == calc_crc else "MISMATCH"}')
        
        # Read partition entries
        f.seek(pe_lba * 512)
        all_entries = f.read(num_parts * part_sz)
        calc_pe_crc = zlib.crc32(all_entries) & 0xFFFFFFFF
        print(f'PE CRC calc: {calc_pe_crc:#010x} {"OK" if pe_crc == calc_pe_crc else "MISMATCH"}')
        
        # List ESP partition
        efi_guid_bytes = bytes.fromhex('28732AC11FF8D211BA4B00A0C93EC93B')
        for i in range(num_parts):
            pe = all_entries[i*part_sz:i*part_sz+part_sz]
            if pe[0:16] == efi_guid_bytes:
                type_name = 'ESP'
            elif pe[0] == 0 and pe[32:40] == bytes(8):
                continue
            else:
                type_name = pe[0:16].hex()[:8] + '...'
            start = struct.unpack_from('<Q', pe, 32)[0]
            end = struct.unpack_from('<Q', pe, 40)[0]
            if start == 0 and end == 0:
                continue
            f16 = ''
            if start > 0:
                f.seek(start * 512)
                bd = f.read(2)
                if bd[0] in (0xEB, 0xE9):
                    f.seek(start * 512 + 510)
                    sigb = f.read(2)
                    if sigb == b'\x55\xaa':
                        f16 = ' (FAT16)'
            size_mb = (end - start + 1) * 512 / 1024 / 1024
            print(f'  Part {i}: {type_name} LBA {start}-{end} ({size_mb:.1f} MB){f16}')
        
        # Check backup GPT at alternate LBA
        f.seek(alt_lba * 512)
        bak = f.read(92)
        bak_sig = bak[0:8].decode('ascii', errors='replace')
        print(f'Backup GPT at LBA {alt_lba}: sig={bak_sig}')
        
        # MBR
        f.seek(0)
        mbr = f.read(512)
        print(f'MBR 55AA: {"YES" if mbr[510:512]==b"\\x55\\xaa" else "NO"}')
        for i in range(4):
            off = 446 + i*16
            bi, pt = mbr[off], mbr[off+4]
            if pt != 0:
                s = struct.unpack('<I', mbr[off+8:off+12])[0]
                print(f'  Part {i}: boot={bi:#04x} type={pt:#04x} start={s}')
