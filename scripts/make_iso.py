"""Create a UEFI-bootable ISOHybrid (file-based El Torito + MBR + GPT).
Works in: QEMU (-cdrom and -drive media=disk), USB (dd/Rufus DD mode)."""

import struct, os, sys, subprocess, shutil, zlib

def read_gpt_partitions(image_path):
    with open(image_path, 'rb') as f:
        f.seek(512)
        hdr = f.read(92)
        if hdr[0:8] != b'EFI PART':
            return [(34, None)]
        part_start = struct.unpack_from('<Q', hdr, 72)[0]
        num_parts  = struct.unpack_from('<I', hdr, 80)[0]
        part_size  = struct.unpack_from('<I', hdr, 84)[0]
        efi_guid = bytes.fromhex('28732AC11FF8D211BA4B00A0C93EC93B')
        parts = []
        f.seek(part_start * 512)
        for i in range(min(num_parts, 128)):
            entry = f.read(part_size)
            if len(entry) < 56: break
            type_guid = entry[0:16]
            start_lba, end_lba, attrs = struct.unpack_from('<QQQ', entry, 32)
            if start_lba == 0: continue
            parts.append({'index': i, 'start_lba': start_lba, 'end_lba': end_lba, 'is_esp': type_guid == efi_guid})
        return parts

def extract_esp(bootimage_path, esp_path):
    parts = read_gpt_partitions(bootimage_path)
    esp = next((p for p in parts if p.get('is_esp')), parts[0] if parts else None)
    with open(bootimage_path, 'rb') as src:
        if esp:
            start, end = esp['start_lba'], esp.get('end_lba')
            src.seek(start * 512)
            size = (end - start + 1) * 512 if end else (os.path.getsize(bootimage_path) - start * 512)
        else:
            start = 34
            src.seek(start * 512)
            size = os.path.getsize(bootimage_path) - start * 512
        data = src.read(size)
    bpb = data[:512]
    has_bpb = bpb[0] in (0xEB, 0xE9) and bpb[510:512] == b'\x55\xAA'
    has_fat16 = bpb[11:13] == b'\x00\x02'
    with open(esp_path, 'wb') as dst:
        dst.write(data)
    print(f"  ESP extracted: {size:,} bytes | FAT16: {'YES' if has_fat16 else 'NO'} | OEM: {bpb[3:11].decode('ascii', errors='replace') if has_bpb else 'N/A'}")

def find_xorriso():
    path = shutil.which('xorriso')
    if path: return path
    try:
        r = subprocess.run(['wsl', 'xorriso', '--version'], capture_output=True, timeout=8, text=True)
        if r.returncode == 0: return 'wsl'
    except: pass
    return None

def to_wsl_path(p):
    for c in 'ABCDEFGHIJKLMNOPQRSTUVWXYZ':
        d = f'{c}:'
        if p.startswith(d): return '/mnt/' + c.lower() + p[2:].replace('\\', '/')
    return p

def iso_paths(p):
    """Convert absolute paths for WSL xorriso, leave flags as-is."""
    return to_wsl_path(p) if p and os.path.isabs(p) else p

def create_iso(esp_path, output_path, version):
    vol_id = f'SARGA_OS_{version}'[:32]
    xorriso = find_xorriso()
    if not xorriso:
        print("  xorriso not found")
        return False

    build_dir = os.path.dirname(esp_path)
    content_dir = os.path.join(build_dir, 'iso_root')
    os.makedirs(content_dir, exist_ok=True)
    with open(os.path.join(content_dir, 'README.txt'), 'w') as f:
        f.write(f'Vahi OS {version} - UEFI Bootable\n')
    shutil.copy2(esp_path, os.path.join(content_dir, 'esp.img'))

    cmd = ['xorriso', '-as', 'mkisofs',
           '-V', vol_id, '-iso-level', '2',
           '-eltorito-alt-boot', '-e', 'esp.img', '-no-emul-boot',
           '-o', output_path, content_dir]
    if xorriso == 'wsl':
        cmd = ['wsl'] + [iso_paths(c) for c in cmd]

    print(f"  Running: {' '.join(cmd)}")
    if subprocess.run(cmd).returncode != 0:
        print("  xorriso failed")
        return False

    size = os.path.getsize(output_path)
    print(f"  ISO created: {size:,} bytes ({size/1024/1024:.1f} MB)")

    with open(output_path, 'r+b') as f:
        data = f.read()
        total_sectors = size // 512

        # Parse root directory for BOOT.CATALOG and ESP.IMG locations
        pvd = data[16 * 2048:17 * 2048]
        root_loc = struct.unpack_from('<I', pvd, 158)[0]
        rd = data[root_loc * 2048:root_loc * 2048 + 2048]
        cat_lbn = esp_lbn = esp_size = None
        off = 0
        while off < len(rd) - 32:
            rec_len = rd[off]
            if rec_len == 0: break
            name_len = rd[off + 32]
            name = rd[off + 33:off + 33 + name_len].decode('ascii', errors='replace').split(';')[0]
            # ponytail: strip trailing dot only when ISO stored NAME.;1 (no-extension file)
            if name.endswith('.'):
                name = name.rstrip('.')
            file_lba = struct.unpack_from('<I', rd, off + 2)[0]
            file_size = struct.unpack_from('<I', rd, off + 10)[0]
            if name.upper() == 'BOOT.CATALOG': cat_lbn = file_lba
            elif name.upper() == 'ESP.IMG': esp_lbn = file_lba; esp_size = file_size
            off += rec_len

        if not esp_lbn or not cat_lbn:
            print("  ERROR: required files not found in ISO")
            return False

        # Fix Boot Record catalog pointer at offset 71 (El Torito spec)
        f.seek(17 * 2048 + 71)
        f.write(struct.pack('<I', cat_lbn))

        esp_lba = esp_lbn * 4
        esp_sectors = esp_size // 512

        print(f"  ESP: RBA={esp_lbn} LBA={esp_lba} sectors={esp_sectors}")

        # MBR: GPT Protective
        mbr = bytearray(512)
        mbr[446] = 0x00
        mbr[447:450] = b'\x00\x02\x00'
        mbr[450] = 0xEE
        mbr[451:454] = b'\xff\xff\xff'
        mbr[454:458] = struct.pack('<I', 1)
        sz = min(total_sectors - 1, 0xFFFFFFFF)
        mbr[458:462] = struct.pack('<I', sz)
        mbr[510:512] = b'\x55\xaa'
        f.seek(0)
        f.write(mbr)

        # GPT header
        gpt_hdr = bytearray(92)
        gpt_hdr[0:8] = b'EFI PART'
        gpt_hdr[8:12] = struct.pack('<I', 0x00010000)
        gpt_hdr[12:16] = struct.pack('<I', 92)
        gpt_hdr[24:32] = struct.pack('<Q', 1)
        gpt_hdr[32:40] = struct.pack('<Q', total_sectors - 1)
        gpt_hdr[40:48] = struct.pack('<Q', 34)
        gpt_hdr[48:56] = struct.pack('<Q', total_sectors - 34)
        gpt_hdr[56:72] = os.urandom(16)
        gpt_hdr[72:80] = struct.pack('<Q', 2)
        gpt_hdr[80:84] = struct.pack('<I', 128)
        gpt_hdr[84:88] = struct.pack('<I', 128)

        # Partition entry: ESP (EFI System GUID)
        efi_guid = bytes.fromhex('28732AC11FF8D211BA4B00A0C93EC93B')
        all_entries = bytearray(128 * 128)
        all_entries[0:16] = efi_guid
        all_entries[16:32] = os.urandom(16)
        all_entries[32:40] = struct.pack('<Q', esp_lba)
        all_entries[40:48] = struct.pack('<Q', esp_lba + esp_sectors - 1)
        name_bytes = 'EFI System Partition'.encode('utf-16-le')
        all_entries[56:128] = name_bytes.ljust(72, b'\x00')[:72]

        entries_crc = zlib.crc32(all_entries) & 0xFFFFFFFF
        gpt_hdr[88:92] = struct.pack('<I', entries_crc)
        gpt_hdr[16:20] = b'\x00\x00\x00\x00'
        gpt_hdr[16:20] = struct.pack('<I', zlib.crc32(gpt_hdr) & 0xFFFFFFFF)

        f.seek(512)
        f.write(gpt_hdr)
        f.seek(1024)
        f.write(all_entries)

        # Backup GPT at end of disk
        last_lba = total_sectors - 1
        f.seek((last_lba - 32) * 512)
        f.write(all_entries)
        backup_hdr = bytearray(gpt_hdr)
        backup_hdr[24:32] = struct.pack('<Q', last_lba)
        backup_hdr[32:40] = struct.pack('<Q', 34)
        backup_hdr[16:20] = b'\x00\x00\x00\x00'
        backup_hdr[16:20] = struct.pack('<I', zlib.crc32(backup_hdr) & 0xFFFFFFFF)
        f.seek(last_lba * 512)
        f.write(backup_hdr)

    shutil.rmtree(content_dir, ignore_errors=True)
    print(f"  Hybrid: MBR=YES GPT=YES (ESP LBA={esp_lba}, {esp_sectors} sectors)")
    return True

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    skyos_root = os.path.dirname(script_dir)

    def find_candidate(patterns):
        for kdir in ['SKYIOUS KERNEL', 'SKYIOUS-KERNEL']:
            for base_rel in [os.path.join(skyos_root, '..', kdir), os.path.join(skyos_root, kdir)]:
                for pat in patterns:
                    p = os.path.join(base_rel, pat)
                    if os.path.exists(p): return p
        return None

    profiles = ['release', 'debug']
    bootimage = find_candidate([f'target/x86_64-vahi/{p}/bootimage-vahi_kernel.bin' for p in profiles])
    if not bootimage: bootimage = find_candidate(['bootimage-vahi_kernel.bin'])
    if not bootimage:
        print("ERROR: bootimage-vahi_kernel.bin not found")
        sys.exit(1)

    version = sys.argv[1] if len(sys.argv) > 1 else '0.6.0'
    release_dir = os.path.join(skyos_root, 'release')
    build_dir = os.path.join(skyos_root, 'build')
    os.makedirs(release_dir, exist_ok=True)
    os.makedirs(build_dir, exist_ok=True)

    esp_path = os.path.join(build_dir, 'esp.img')
    iso_path = os.path.join(release_dir, f'skyos-{version}.iso')

    print(f"Bootimage: {bootimage}")
    print(f"Version:   {version}")
    print(f"ESP:       {esp_path}")
    print(f"ISO:       {iso_path}")

    print("\n1. Extracting ESP...")
    extract_esp(bootimage, esp_path)

    print("2. Creating ISO...")
    if not create_iso(esp_path, iso_path, version):
        sys.exit(1)

    size = os.path.getsize(iso_path)
    print(f"\nSUCCESS: {iso_path} ({size/1024/1024:.1f} MB)")
    print("Boot with: qemu-system-x86_64 -bios OVMF.fd -cdrom <iso>")
    print("Or:        qemu-system-x86_64 -bios OVMF.fd -drive file=<iso>,format=raw,media=disk")
    print("Flash to USB: dd if=<iso> of=/dev/sdX bs=4M status=progress")

if __name__ == '__main__':
    main()
