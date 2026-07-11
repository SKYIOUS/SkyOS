#!/usr/bin/env python3
"""
SARGA OS Installer ISO Builder (isohybrid)

Creates a UEFI-bootable ISO that is simultaneously a valid GPT disk image
and a valid ISO 9660 filesystem. Boots via:
  - USB:  GPT ESP partition (bootloader crate EFI + kernel on FAT16)
  - CD:   El Torito (best-effort, see note at bottom)
"""

import struct, os, sys, zlib

SECTOR = 2048
BLOCK = 512

def extract_esp(path):
    with open(path, 'rb') as f:
        f.seek(512); gpt = f.read(92)
        if gpt[0:8] != b'EFI PART': return
        plba = struct.unpack('<Q', gpt[72:80])[0]
        np = struct.unpack('<I', gpt[80:84])[0]
        ps = struct.unpack('<I', gpt[84:88])[0]
        f.seek(plba * BLOCK)
        for _ in range(np):
            e = f.read(ps)
            if e[0] == 0: continue
            sl, el = struct.unpack('<Q', e[32:40])[0], struct.unpack('<Q', e[40:48])[0]
            if e[0:16] == bytes.fromhex('28732ac11ff8d211ba4b00a0c93ec93b'):
                f.seek(sl * BLOCK)
                return f.read((el - sl + 1) * BLOCK)

def pad(b, n):
    r = len(b) % n
    return b if r == 0 else b + bytes(n - r)

def main():
    bootimg = 'C:\\Users\\nanda\\bootimg.bin'
    alt = 'C:\\Users\\nanda\\Desktop\\Github\\SKYIOUS KERNEL\\target\\x86_64-vahi\\debug\\bootimage-vahi_kernel.bin'
    if not os.path.exists(bootimg) and os.path.exists(alt): bootimg = alt
    if not os.path.exists(bootimg): print("ERROR: bootimage not found"); sys.exit(1)

    out = 'C:\\Users\\nanda\\SkyOS-Installer.iso'
    print("=== SARGA OS Installer ISO (isohybrid) ===")

    print("[1/2] Extracting ESP...")
    esp = extract_esp(bootimg)
    if not esp: print("ERROR: Could not extract ESP"); sys.exit(1)
    print(f"  ESP: {len(esp)} bytes ({len(esp)//1024} KB)")

    with open(bootimg, 'rb') as f: bootimg_data = f.read()
    readme_text = b"SARGA OS Installer ISO\nBoot in UEFI mode.\nFor USB: write BOOTIMAGE.BIN with dd or Rufus.\n"

    ESP_SECTORS = len(esp) // BLOCK          # in 512-byte blocks
    ESP_ISO_SECS = (len(esp) + SECTOR - 1) // SECTOR
    BOOTIMG_SECS = (len(bootimg_data) + SECTOR - 1) // SECTOR
    RDME_SECS = (len(readme_text) + SECTOR - 1) // SECTOR

    # === Layout (ISO 2048-byte sectors) ===
    # 0-15:   System area (GPT: MBR LBA0, GPT header LBA1, entries LBA2-33, gap to 67)
    # 16:     PVD
    # 17:     El Torito Boot Record
    # 18:     Terminator
    # 19:     Boot Catalog
    # 20+:    ESP (FAT16) — aligned to ISO sector boundary
    # 20+N+:  BOOTIMAGE.BIN (full GPT disk as file)
    # 20+N+M: README.TXT

    ESP_ISO_LBA = 26                     # ISO sector where ESP data begins (after metadata)
    IMG_ISO_LBA = ESP_ISO_LBA + ESP_ISO_SECS
    RDME_ISO_LBA = IMG_ISO_LBA + BOOTIMG_SECS
    # Reserve 9 ISO sectors at end for backup GPT (33 blocks x 512 = 16896, ceil/2048 = 9)
    BACKUP_RESERVE_SECS = 9
    TOTAL_SECS = RDME_ISO_LBA + RDME_SECS + BACKUP_RESERVE_SECS

    TOTAL_BYTES = TOTAL_SECS * SECTOR

    # GPT: ESP partition LBA in 512-byte block units
    ESP_LBA_BLOCK = ESP_ISO_LBA * SECTOR // BLOCK   # = 26 * 4 = 104
    TOTAL_BLOCKS = TOTAL_BYTES // BLOCK

    print(f"\n[2/2] Building isohybrid image ({TOTAL_BYTES/1024/1024:.0f} MB)...")
    print(f"  ESP FAT at ISO sector {ESP_ISO_LBA} (LBA {ESP_LBA_BLOCK}), {ESP_SECTORS} sectors")
    print(f"  BOOTIMAGE.BIN at ISO sector {IMG_ISO_LBA}")
    print(f"  README.TXT at ISO sector {RDME_ISO_LBA}")

    iso = bytearray(TOTAL_BYTES)

    # Partition entry (ESP)
    pe = bytearray(128)
    pe[0:16] = bytes.fromhex('28732ac11ff8d211ba4b00a0c93ec93b')
    pe[32:40] = struct.pack('<Q', ESP_LBA_BLOCK)
    pe[40:48] = struct.pack('<Q', ESP_LBA_BLOCK + ESP_SECTORS - 1)
    import uuid
    pe[16:32] = uuid.uuid4().bytes_le
    pe[56:128] = 'ESP\0'.encode('utf-16-le').ljust(72, b'\x00')
    pe_crc = zlib.crc32(bytes(pe * 0) + pe) & 0xFFFFFFFF  # CRC over 128 entries, only first non-zero

    # Build full partition entries array and compute joint CRC
    entries_blob = bytearray()
    for i in range(128):
        if i == 0: entries_blob += pe
        else: entries_blob += b'\x00' * 128
    entries_crc = zlib.crc32(bytes(entries_blob)) & 0xFFFFFFFF

    # Write primary partition entries at LBA 2-33
    for i in range(128):
        off = 2 * BLOCK + i * 128
        if i == 0: iso[off:off+128] = pe

    # Primary GPT header at LBA 1
    def make_gpt_header(my_lba, alt_lba, pe_lba, entries_crc_val):
        h = bytearray(BLOCK)
        h[0:8] = b'EFI PART'
        h[8:12] = struct.pack('<I', 0x00010000)
        h[12:16] = struct.pack('<I', 92)
        h[24:32] = struct.pack('<Q', my_lba)
        h[32:40] = struct.pack('<Q', alt_lba)
        h[40:48] = struct.pack('<Q', 34)
        h[48:56] = struct.pack('<Q', TOTAL_BLOCKS - 34)
        h[56:72] = b'\x00' * 16
        h[72:80] = struct.pack('<Q', pe_lba)
        h[80:84] = struct.pack('<I', 128)
        h[84:88] = struct.pack('<I', 128)
        h[88:92] = struct.pack('<I', entries_crc_val)
        hdr_crc = zlib.crc32(bytes(h[:92])) & 0xFFFFFFFF
        h[16:20] = struct.pack('<I', hdr_crc)
        return h

    iso[BLOCK:BLOCK*2] = make_gpt_header(1, TOTAL_BLOCKS - 1, 2, entries_crc)

    # ── Backup GPT at end of disk ────────────────────────────
    BACKUP_PE_LBA = TOTAL_BLOCKS - 33
    BACKUP_GPT_LBA = TOTAL_BLOCKS - 1
    for i in range(128):
        off = BACKUP_PE_LBA * BLOCK + i * 128
        if i == 0: iso[off:off+128] = pe
    iso[BACKUP_GPT_LBA*BLOCK:BACKUP_GPT_LBA*BLOCK+BLOCK] = make_gpt_header(BACKUP_GPT_LBA, 1, BACKUP_PE_LBA, entries_crc)

    # ── ISO 9660 Volume Descriptors ─────────────────────────
    # Sector 16: PVD
    pvd = bytearray(SECTOR)
    pvd[0] = 1
    pvd[1:6] = b'CD001'; pvd[7] = 1
    pvd[8:40] = b'SARGA OS'.ljust(32, b' ')
    pvd[40:72] = b'SARGA_OS_INSTALL'.ljust(32, b' ')
    pvd[80:84] = struct.pack('<I', TOTAL_SECS)
    pvd[84:88] = struct.pack('>I', TOTAL_SECS)
    pvd[120:124] = struct.pack('<H', 1) + struct.pack('>H', 1)
    pvd[124:128] = struct.pack('<H', 1) + struct.pack('>H', 1)
    pvd[128:132] = struct.pack('<H', SECTOR) + struct.pack('>H', SECTOR)
    # Path tables: L at 20, M at 21
    pt_l = bytearray()
    pt_l += bytes([1, 0]) + struct.pack('<I', 22) + struct.pack('<H', 1) + b'\x00'
    pt_l += bytes([4, 0]) + struct.pack('<I', 24) + struct.pack('<H', 1) + b'BOOT'
    if len(b'BOOT') % 2 == 0: pt_l += b'\x00'
    pt_m = bytearray()
    pt_m += bytes([1, 0]) + struct.pack('>I', 22) + struct.pack('>H', 1) + b'\x00'
    pt_m += bytes([4, 0]) + struct.pack('>I', 24) + struct.pack('>H', 1) + b'BOOT'
    if len(b'BOOT') % 2 == 0: pt_m += b'\x00'
    pvd[132:136] = struct.pack('<I', len(pt_l))
    pvd[136:140] = struct.pack('>I', len(pt_m))
    pvd[140:144] = struct.pack('<I', 20)   # L path table
    pvd[144:148] = struct.pack('<I', 0)
    pvd[148:152] = struct.pack('>I', 21)   # M path table
    pvd[152:156] = struct.pack('>I', 0)

    # Root directory entry
    def drec(extent, dlen, flags, name, flen=None):
        if flen is None: flen = len(name)
        nb = name if isinstance(name, bytes) else name.encode('ascii')
        r = bytearray(33 + len(nb))
        r[0] = len(r); r[1] = 0
        r[2:6] = struct.pack('<I', extent); r[6:10] = struct.pack('>I', extent)
        r[10:14] = struct.pack('<I', dlen); r[14:18] = struct.pack('>I', dlen)
        r[18:25] = bytes(7)
        r[25] = flags; r[26] = 0; r[27] = 0
        r[28:30] = struct.pack('<H', 1) + struct.pack('>H', 1)
        r[31] = flen; r[32:32+flen] = nb[:flen]
        if (33 + flen) % 2: r.append(0)
        r[0] = len(r)
        return bytes(r)

    root_entries = bytearray()
    root_entries += drec(22, SECTOR, 0x02, b'\x00')          # .
    root_entries += drec(22, SECTOR, 0x02, b'\x01')          # ..
    root_entries += drec(24, SECTOR, 0x02, b'BOOT')          # BOOT dir
    root_entries += drec(IMG_ISO_LBA, BOOTIMG_SECS*SECTOR, 0x00, b'BOOTIMAGE.BIN')
    root_entries += drec(RDME_ISO_LBA, RDME_SECS*SECTOR, 0x00, b'README.TXT')
    root_pad = pad(root_entries, SECTOR)

    rec = drec(22, len(root_pad), 0x02, b'\x00')
    pvd[156:156+len(rec)] = rec

    iso[16*SECTOR:17*SECTOR] = pvd

    # Sector 17: Boot Record
    br = bytearray(SECTOR)
    br[0] = 0
    br[1:6] = b'CD001'; br[7] = 1
    br[8:40] = b'EL TORITO SPECIFICATION'.ljust(32, b'\x00')
    br[72:76] = struct.pack('<I', 19)  # catalog ISO sector 19
    iso[17*SECTOR:18*SECTOR] = br

    # Sector 18: Terminator
    vdst = bytearray(SECTOR)
    vdst[0] = 255; vdst[1:6] = b'CD001'; vdst[7] = 1
    iso[18*SECTOR:19*SECTOR] = vdst

    # Sector 19: Boot Catalog
    cat = bytearray(SECTOR)
    cat[0] = 1; cat[1] = 0xEF         # validation entry, UEFI platform
    cat[4:28] = b'SKYIOUS'.ljust(24, b'\x00')
    cksum = sum(cat[j] | (cat[j+1] << 8) for j in range(0, 32, 2)) & 0xFFFF
    cat[28] = (-cksum) & 0xFF; cat[29] = ((-cksum) >> 8) & 0xFF
    cat[32] = 0x88; cat[33] = 0        # initial entry: bootable, no emulation
    cat[38:40] = struct.pack('<H', ESP_SECTORS)   # sector count (512-byte)
    cat[40:44] = struct.pack('<I', ESP_ISO_LBA)    # load RBA (ISO sector)
    iso[19*SECTOR:20*SECTOR] = cat

    # Sector 20-23: Path tables, directory records
    iso[20*SECTOR:21*SECTOR] = pad(pt_l, SECTOR)[:SECTOR]
    iso[21*SECTOR:22*SECTOR] = pad(pt_m, SECTOR)[:SECTOR]
    iso[22*SECTOR:23*SECTOR] = root_pad[:SECTOR]
    # BOOT directory
    boot_dir = bytearray()
    boot_dir += drec(24, SECTOR, 0x02, b'\x00')                 # .
    boot_dir += drec(22, len(root_pad), 0x02, b'\x01')          # ..
    boot_dir += drec(ESP_ISO_LBA, ESP_ISO_SECS*SECTOR, 0x00, b'BOOTX64.FAT')
    iso[24*SECTOR:25*SECTOR] = pad(boot_dir, SECTOR)

    # ── File data ────────────────────────────────────────────
    off = ESP_ISO_LBA * SECTOR
    iso[off:off+len(esp)] = esp

    off = IMG_ISO_LBA * SECTOR
    iso[off:off+len(bootimg_data)] = bootimg_data

    off = RDME_ISO_LBA * SECTOR
    iso[off:off+len(readme_text)] = readme_text

    # ── Write ────────────────────────────────────────────────
    with open(out_path := out, 'wb') as f:
        f.write(iso)

    sz = os.path.getsize(out_path)
    print(f"\nSUCCESS: {out_path} ({sz/1024/1024:.1f} MB)")
    print(f"  GPT ESP partition: LBA {ESP_LBA_BLOCK}-{ESP_LBA_BLOCK+ESP_SECTORS-1}")
    print(f"  El Torito: catalog at ISO sector 19, boot image at ISO sector {ESP_ISO_LBA}")
    print()
    print("Boot test:")
    print(f"  qemu-system-x86_64 -bios OVMF.fd -drive file={os.path.basename(out_path)},format=raw,if=ide")
    print(f"  qemu-system-x86_64 -bios OVMF.fd -cdrom {os.path.basename(out_path)}")
    print()
    print("NOTE: When booting as CD-ROM via -cdrom, some OVMF versions may not")
    print("recognize the El Torito FAT filesystem. Use -drive for guaranteed boot.")
    print("The image also works when written to USB with dd / Rufus.")

if __name__ == '__main__':
    main()
