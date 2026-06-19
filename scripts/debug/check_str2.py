import struct

with open(
    "C:\\Users\\nanda\\Desktop\\Github\\SkyOS\\target\\x86_64-sarga\\release\\init",
    "rb",
) as f:
    data = f.read()
    e_phoff = struct.unpack("<Q", data[0x20:0x28])[0]
    e_phentsize = struct.unpack("<H", data[0x36:0x38])[0]
    e_phnum = struct.unpack("<H", data[0x38:0x3A])[0]
    for i in range(e_phnum):
        off = e_phoff + i * e_phentsize
        p_type = struct.unpack("<I", data[off : off + 4])[0]
        if p_type == 1:
            p_offset = struct.unpack("<Q", data[off + 8 : off + 16])[0]
            p_vaddr = struct.unpack("<Q", data[off + 16 : off + 24])[0]
            p_filesz = struct.unpack("<Q", data[off + 32 : off + 40])[0]
            p_flags = struct.unpack("<I", data[off + 4 : off + 8])[0]
            flags_str = ""
            if p_flags & 1:
                flags_str += "X"
            if p_flags & 2:
                flags_str += "W"
            if p_flags & 4:
                flags_str += "R"
            seg = data[p_offset : p_offset + p_filesz]
            for s in [
                b"/etc/init.cfg",
                b"OK\n",
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                b"FAIL",
            ]:
                pos = seg.find(s)
                if pos >= 0:
                    va = p_vaddr + pos
                    print(f'LOAD {flags_str}: "{s.decode()}" at VA 0x{va:x}')
                else:
                    print(f'LOAD {flags_str}: "{s.decode()}" NOT FOUND')
            print()
