with open(
    "C:\\Users\\nanda\\Desktop\\Github\\SkyOS\\target\\x86_64-sarga\\release\\init",
    "rb",
) as f:
    data = f.read()
    target = b"/etc/init.cfg"
    # Search byte by byte
    for i in range(len(data) - len(target) + 1):
        if data[i : i + len(target)] == target:
            print(f"FOUND /etc/init.cfg at offset 0x{i:x}")
    # Search for partial
    for part in [b"/etc", b"init.cfg", b"etc/init"]:
        pos = data.find(part)
        if pos >= 0:
            print(
                f'Partial "{part.decode()}" at offset 0x{pos:x}: {data[max(0, pos - 4) : pos + len(part) + 4]}'
            )
        else:
            print(f'Partial "{part.decode()}" NOT FOUND')
    # Search for the null-terminated or length-prefixed version
    # Some V1 format uses 1-byte length prefix
    for i in range(len(data) - 20):
        if (
            data[i] == 0x0D and data[i : i + 14] == b"\x0d/etc/init.cfg"
        ):  # 13-char string with length prefix
            print(f"[LEN13] /etc/init.cfg at 0x{i:x}")
        if (
            data[i] == 14 and data[i : i + 15] == b"\x0e/etc/init.cfg\x00"
        ):  # null terminated with length
            print(f"[LEN14] /etc/init.cfg at 0x{i:x}")
    # Also dump the .rodata portion around the main string areas
    for area_start in [0x3468, 0x3600]:
        area = data[area_start : area_start + 0x200]
        print(f"\nArea at file offset 0x{area_start:x}:")
        # Show as hex bytes
        for j in range(0, min(len(area), 0x200), 16):
            hex_str = " ".join(f"{b:02x}" for b in area[j : j + 16])
            ascii_str = "".join(
                chr(b) if 32 <= b < 127 else "." for b in area[j : j + 16]
            )
            va = 0x402468 + (area_start - 0x3468) + j
            print(f"  0x{va:x}: {hex_str}  {ascii_str}")
