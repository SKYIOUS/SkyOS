with open(
    "C:\\Users\\nanda\\Desktop\\Github\\SkyOS\\target\\x86_64-sarga\\release\\init",
    "rb",
) as f:
    data = f.read()
    # Search for the bytes of "/etc/init.cfg" in the WR segments (data)
    import struct

    e_phoff = struct.unpack("<Q", data[0x20:0x28])[0]
    e_phentsize = struct.unpack("<H", data[0x36:0x38])[0]
    e_phnum = struct.unpack("<H", data[0x38:0x3A])[0]

    # Check each LOAD segment for the string
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack("<I", data[off : off + 4])[0]
        if p_type == 1:
            p_offset = struct.unpack("<Q", data[off + 8 : off + 16])[0]
            p_filesz = struct.unpack("<Q", data[off + 32 : off + 40])[0]
            seg = data[p_offset : p_offset + p_filesz]
            # Search for /etc/init
            for j in range(len(seg) - 9):
                # Check 9 bytes starting at j
                chunk = seg[j : j + 9]
                if chunk[:8] == b"/etc/ini" and chunk[8] in (ord("t"), 0x48):
                    print(f"Match at file offset 0x{p_offset + j:x}: chunk={chunk}")
                # Also check for partial matches
                if chunk[:4] == b"/etc":
                    print(f"/etc at file offset 0x{p_offset + j:x}: {chunk}")
    # Additionally look at .data segment content in hex
    print()
    print("Segment 3 (WR, .data) full hex dump:")
    seg3_off = 0x3A70
    seg3 = data[seg3_off : seg3_off + 0x100]
    for j in range(0, len(seg3), 16):
        hex_str = " ".join(f"{b:02x}" for b in seg3[j : j + 16])
        ascii_str = "".join(chr(b) if 32 <= b < 127 else "." for b in seg3[j : j + 16])
        va = 0x402A70 + j
        print(f"  0x{va:x}: {hex_str}  {ascii_str}")
