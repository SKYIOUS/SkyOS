"""Deep inspect BOOTX64.EFI header."""
with open('build/BOOTX64.EFI', 'rb') as f:
    data = f.read()

# Check MZ header
print(f'MZ: {data[0:2]}')

# e_lfanew at offset 0x3C
pe_off = int.from_bytes(data[0x3C:0x40], 'little')
print(f'PE offset (e_lfanew): {pe_off:#x}')

if pe_off < len(data):
    pe_sig = data[pe_off:pe_off+4]
    print(f'PE signature at {pe_off:#x}: {pe_sig}')

    if pe_sig == b'PE\x00\x00':
        # Read PE header
        machine = int.from_bytes(data[pe_off+4:pe_off+6], 'little')
        num_sections = int.from_bytes(data[pe_off+6:pe_off+8], 'little')
        timestamp = int.from_bytes(data[pe_off+8:pe_off+12], 'little')
        print(f'Machine: {machine:#06x} ({machine})')
        print(f'Sections: {num_sections}')
        print(f'Timestamp: {timestamp}')

        # Optional header (PE32+ for x64)
        opt_hdr_size = int.from_bytes(data[pe_off+20:pe_off+22], 'little')
        pe32plus_magic = int.from_bytes(data[pe_off+24:pe_off+26], 'little')
        print(f'Optional header size: {opt_hdr_size}')
        print(f'PE magic: {pe32plus_magic:#06x} (0x20b = PE32+)')

        # Subsystem
        if pe32plus_magic == 0x20b:
            subsystem = int.from_bytes(data[pe_off+68:pe_off+70], 'little')
            print(f'Subsystem: {subsystem} (10=EFI boot, 11=runtime, 12=EFI app)')

        # Section headers
        sections_off = pe_off + 24 + opt_hdr_size
        for i in range(num_sections):
            sec = data[sections_off + i*40:sections_off + (i+1)*40]
            name = sec[:8].decode('ascii', errors='replace').rstrip('\x00')
            vaddr = int.from_bytes(sec[12:16], 'little')
            vsize = int.from_bytes(sec[16:20], 'little')
            raw_off = int.from_bytes(sec[20:24], 'little')
            raw_sz = int.from_bytes(sec[24:28], 'little')
            print(f'  Section {name}: vaddr={vaddr:#x} vsize={vsize} raw={raw_off} rawsize={raw_sz}')
    else:
        # Dump bytes around PE offset for debugging
        start = max(0, pe_off - 16)
        end = min(len(data), pe_off + 64)
        for i in range(start, end, 16):
            hexb = ' '.join(f'{b:02x}' for b in data[i:i+16])
            asci = ''.join(chr(b) if 32 <= b < 127 else '.' for b in data[i:i+16])
            print(f'  {i:08x}: {hexb:48s} {asci}')
else:
    print(f'PE offset {pe_off} beyond file size {len(data)}')
    # Dump bytes at known locations
    for off in [0x30, 0x38, 0x3C, 0x40, 0x50, 0x60, 0x70, 0x78, 0x80]:
        if off + 8 <= len(data):
            hexb = ' '.join(f'{b:02x}' for b in data[off:off+8])
            print(f'  {off:08x}: {hexb}')

# Bootloader crate specific: the UEFI application should be in PE format
# Search for 'PE' in the file
for i in range(len(data) - 4):
    if data[i:i+2] == b'PE' and data[i+2:i+4] == b'\x00\x00':
        print(f'\nFound PE signature at offset {i:#x}')
        print(f'  Around it: {" ".join(f"{b:02x}" for b in data[i-8:i+64])}')
        break
else:
    print('\nNo PE signature found anywhere in file!')
