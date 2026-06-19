import struct

with open(
    r"C:\Users\nanda\Desktop\Github\SkyOS\target\x86_64-sarga\release\init", "rb"
) as f:
    data = f.read(64)
    print("Magic:", data[:4])
    if data[:4] == b"\x7fELF":
        ei_class = data[4]
        ei_data = data[5]
        e_type = struct.unpack("<H", data[16:18])[0]
        e_machine = struct.unpack("<H", data[18:20])[0]
        e_entry = struct.unpack("<Q", data[24:32])[0]
        e_phoff = struct.unpack("<Q", data[32:40])[0]
        e_phnum = struct.unpack("<H", data[56:58])[0]

        type_str = {2: "EXEC", 3: "DYN (PIE)"}.get(e_type, str(e_type))
        print("Class:", "64-bit" if ei_class == 2 else "32-bit")
        print("Data:", "LE" if ei_data == 1 else "BE")
        print("Type:", type_str)
        print("Machine:", "x86_64" if e_machine == 0x3E else hex(e_machine))
        print("Entry:", hex(e_entry))
        print("PH offset:", hex(e_phoff), "num:", e_phnum)

        f.seek(e_phoff)
        for i in range(e_phnum):
            ph = f.read(56)
            p_type = struct.unpack("<I", ph[0:4])[0]
            p_flags = struct.unpack("<I", ph[4:8])[0]
            p_offset = struct.unpack("<Q", ph[8:16])[0]
            p_vaddr = struct.unpack("<Q", ph[16:24])[0]
            p_paddr = struct.unpack("<Q", ph[24:32])[0]
            p_filesz = struct.unpack("<Q", ph[32:40])[0]
            p_memsz = struct.unpack("<Q", ph[40:48])[0]
            p_align = struct.unpack("<Q", ph[48:56])[0]
            type_map = {
                1: "LOAD",
                2: "DYNAMIC",
                4: "NOTE",
                0x6474E550: "GNU_EH_FRAME",
                0x6474E551: "GNU_STACK",
                0x6474E552: "GNU_RELRO",
            }
            p_type_str = type_map.get(p_type, hex(p_type))
            print(
                "  PH[{}]: type={} flags={} vaddr={} filesz={} memsz={} align={}".format(
                    i, p_type_str, p_flags, hex(p_vaddr), p_filesz, p_memsz, p_align
                )
            )

        # Check if it's static (no INTERP)
        f.seek(e_phoff)
        has_interp = False
        for i in range(e_phnum):
            ph = f.read(56)
            p_type = struct.unpack("<I", ph[0:4])[0]
            if p_type == 3:  # PT_INTERP
                has_interp = True
                break
        print("Has INTERP (dynamic):", has_interp)
