"""Extract and verify BOOTX64.EFI."""
with open('build/esp.img', 'rb') as f:
    f.seek(86528)
    data = f.read(156672)
    with open('build/BOOTX64.EFI', 'wb') as out:
        out.write(data)

import os
sz = os.path.getsize('build/BOOTX64.EFI')
print(f'Extracted BOOTX64.EFI: {sz} bytes')

with open('build/BOOTX64.EFI', 'rb') as f:
    hdr = f.read(2)
    print(f'MZ header: {"YES" if hdr == b"MZ" else "NO"}')
    f.seek(0x3C)
    pe_off = int.from_bytes(f.read(4), 'little')
    f.seek(pe_off)
    pe_sig = f.read(4)
    print(f'PE signature at {pe_off:#x}: {"YES" if pe_sig == b"PE\\x00\\x00" else "NO"}')
    machine = int.from_bytes(f.read(2), 'little')
    print(f'Machine: {machine:#06x}')
    f.seek(pe_off + 68)
    subsystem = int.from_bytes(f.read(2), 'little')
    print(f'Subsystem: {subsystem}')
