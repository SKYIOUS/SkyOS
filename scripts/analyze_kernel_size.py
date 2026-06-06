"""analyze_kernel_size.py — Break down kernel ELF binary sizes per section."""
import struct, os, sys

def analyze_elf(path):
    if not os.path.exists(path):
        print(f"File not found: {path}")
        return
    size = os.path.getsize(path)
    with open(path, 'rb') as f:
        data = f.read()
    if data[:4] != b'\x7fELF':
        print(f"Not an ELF file: {path}")
        return
    is_64 = data[4] == 2
    is_le = data[5] == 1
    endian = '<' if is_le else '>'
    if is_64:
        shoff = struct.unpack_from(endian + 'Q', data, 0x28)[0]
        shentsize = struct.unpack_from(endian + 'H', data, 0x3A)[0]
        shnum = struct.unpack_from(endian + 'H', data, 0x3C)[0]
        shstrndx = struct.unpack_from(endian + 'H', data, 0x3E)[0]
    else:
        shoff = struct.unpack_from(endian + 'I', data, 0x20)[0]
        shentsize = struct.unpack_from(endian + 'H', data, 0x2E)[0]
        shnum = struct.unpack_from(endian + 'H', data, 0x30)[0]
        shstrndx = struct.unpack_from(endian + 'H', data, 0x32)[0]
    strtab_off = shoff + shstrndx * shentsize
    if is_64:
        strtab_sh_offset = struct.unpack_from(endian + 'Q', data, strtab_off + 0x18)[0]
        strtab_sh_size = struct.unpack_from(endian + 'Q', data, strtab_off + 0x20)[0]
    else:
        strtab_sh_offset = struct.unpack_from(endian + 'I', data, strtab_off + 0x10)[0]
        strtab_sh_size = struct.unpack_from(endian + 'I', data, strtab_off + 0x14)[0]
    strtab = data[strtab_sh_offset:strtab_sh_offset + strtab_sh_size]
    sections = []
    for i in range(shnum):
        off = shoff + i * shentsize
        if is_64:
            name_idx = struct.unpack_from(endian + 'I', data, off)[0]
            sh_type = struct.unpack_from(endian + 'I', data, off + 4)[0]
            sh_flags = struct.unpack_from(endian + 'Q', data, off + 8)[0]
            sh_addr = struct.unpack_from(endian + 'Q', data, off + 0x10)[0]
            sh_offset = struct.unpack_from(endian + 'Q', data, off + 0x18)[0]
            sh_size = struct.unpack_from(endian + 'Q', data, off + 0x20)[0]
        else:
            name_idx = struct.unpack_from(endian + 'I', data, off)[0]
            sh_type = struct.unpack_from(endian + 'I', data, off + 4)[0]
            sh_flags = struct.unpack_from(endian + 'I', data, off + 8)[0]
            sh_addr = struct.unpack_from(endian + 'I', data, off + 0x0C)[0]
            sh_offset = struct.unpack_from(endian + 'I', data, off + 0x10)[0]
            sh_size = struct.unpack_from(endian + 'I', data, off + 0x14)[0]
        name = strtab[name_idx:strtab.find(b'\0', name_idx)].decode('latin-1', errors='replace')
        if sh_type != 0 and sh_size > 0:
            sections.append((name, sh_size, sh_addr, sh_flags))
    total = sum(s[1] for s in sections)
    print(f"=== ELF Section Size Analysis ===")
    print(f"File:      {path}")
    print(f"Total:     {size:,} bytes ({size/1024:.1f} KB)")
    print(f"Sum secs:  {total:,} bytes ({total/1024:.1f} KB)")
    print(f"{'Section':<25} {'Size':>10} {'Pct':>6}  {'Type'}")
    print("-" * 50)
    for name, sz, addr, flags in sorted(sections, key=lambda x: -x[1]):
        typ = []
        if flags & 0x1: typ.append("WR")
        if flags & 0x2: typ.append("ALLOC")
        if flags & 0x4: typ.append("EXEC")
        pct = sz / total * 100 if total else 0
        print(f"{name:<25} {sz:>10,} {pct:>5.1f}%  {','.join(typ)}")

if __name__ == '__main__':
    paths = sys.argv[1:] if len(sys.argv) > 1 else [
        "../SKYIOUS KERNEL/kernel/target/x86_64-unknown-none/debug/vahi_kernel",
    ]
    for p in paths:
        analyze_elf(p)
        print()
