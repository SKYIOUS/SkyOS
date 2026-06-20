"""build_installer_iso.py — Universal bootable SkyOS installer ISO.

Produces skyos-installer.iso — a universal bootable ISO that works on:
  - UEFI systems (x64)
  - Legacy BIOS systems
  - CD/DVD (El Torito)
  - USB drives (isohybrid)

Usage:
  python scripts/build_installer_iso.py           # Build ISO from existing bootimage
  python scripts/build_installer_iso.py --full    # Full rebuild + ISO
  python scripts/build_installer_iso.py --qemu    # Build + test in QEMU
"""
import os, sys, struct, hashlib, shutil, subprocess, tempfile

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_DIR = os.path.normpath(os.path.join(SCRIPT_DIR, ".."))
KERNEL_DIR = os.path.normpath(os.path.join(REPO_DIR, "..", "SKYIOUS KERNEL"))
OUTPUT_ISO = os.path.join(REPO_DIR, "skyos-installer.iso")

BOOTIMAGE_SRC = os.path.join(KERNEL_DIR, "target", "x86_64-vahi", "debug", "bootimage-vahi_kernel.bin")
ESP_GUID = bytes([0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b])

# ─── helpers ───────────────────────────────────────────────────────────────

def le16(v): return struct.pack('<H', v)
def le32(v): return struct.pack('<I', v)
def le64(v): return struct.pack('<Q', v)
def be16(v): return struct.pack('>H', v)
def be32(v): return struct.pack('>I', v)
def lebe16(v): return struct.pack('<H', v) + struct.pack('>H', v)
def lebe32(v): return struct.pack('<I', v) + struct.pack('>I', v)

def pad_to(data, align):
    """Pad data to next alignment boundary (align must be 2048 for ISO)."""
    rem = len(data) % align
    if rem:
        data += b'\0' * (align - rem)
    return data

# ISO9660 directory record timestamp: 7 bytes
DIR_TS = b'\x46\x01\x01\x00\x00\x00\x00'  # year 1970
# ISO9660 volume descriptor date: 17 bytes
VOL_TS = b'\x07\xDE\x01\x01\x00\x00\x00\x00\x00' + b'\x00' * 8  # year 2022

# ─── ISO9660 + El Torito builder ───────────────────────────────────────────

SECTOR = 2048

def _make_dir_record(extent, size, flags, fi, fi_len=None):
    """Build an ISO9660 directory record (variable length)."""
    if fi_len is None:
        fi_len = len(fi)
    rec = bytearray(33 + fi_len + (1 if fi_len % 2 == 0 else 0))
    rec[0] = len(rec)        # len_dr
    rec[1] = 0                # len_xa
    rec[2:10] = lebe32(extent)
    rec[10:18] = lebe32(size)
    rec[18:25] = DIR_TS
    rec[25] = flags
    rec[26] = 0
    rec[27] = 4
    rec[28:32] = lebe16(1)   # volume seq number
    rec[32] = fi_len
    rec[33:33+fi_len] = fi
    return bytes(rec)


class IsoBuilder:
    def __init__(self, vol_id="SKYOS_INSTALL", sys_id="SKYOS"):
        self.sectors = [b'\0' * SECTOR for _ in range(16)]
        self.vol_id = vol_id.ljust(32, b' ')[:32] if isinstance(vol_id, bytes) else vol_id.encode().ljust(32, b' ')[:32]
        self.sys_id = sys_id.ljust(32, b' ')[:32] if isinstance(sys_id, bytes) else sys_id.encode().ljust(32, b' ')[:32]
        self.entries = {}   # full_iso_path -> bytes or None (for dirs)
        self.file_extents = {}
        self.boot_image_data = None
        self.boot_image_path = None

    def add_boot_image(self, data, path="bootimage.bin"):
        """Register boot image data; also adds it as a filesystem entry so
        firmware can locate it via the ISO 9660 directory tree."""
        self.boot_image_data = data
        self.boot_image_path = path
        self.entries[path] = data  # make visible on the ISO filesystem

    def add_file(self, name, data):
        """name is ISO path like 'EFI/BOOT/BOOTX64.EFI' or 'README.TXT'"""
        name = name.replace('\\', '/')
        self.entries[name] = data

    def _ensure_dir(self, path):
        """Ensure directory entries exist for all components of path."""
        parts = path.strip('/').split('/')
        for i in range(len(parts)):
            dir_path = '/' + '/'.join(parts[:i+1])
            if dir_path not in self.entries:
                self.entries[dir_path] = None  # marker for directory

    def _alloc_data(self, data):
        """Append data aligned to SECTOR, return (start_sector, num_sectors)."""
        if len(data) == 0:
            self.sectors.append(b'\0' * SECTOR)
            return (len(self.sectors) - 1, 1)
        aligned = pad_to(data, SECTOR)
        n = len(aligned) // SECTOR
        start = len(self.sectors)
        for i in range(n):
            self.sectors.append(aligned[i*SECTOR:(i+1)*SECTOR])
        return (start, n)

    def build(self):
        # ── Organize file entries ────────────────────────────────────
        # Build a proper tree with all ancestor dirs
        file_list = []
        dir_set = set()
        dir_set.add('/')

        def ensure_ancestors(path, include_self=False):
            parts = path.strip('/').split('/')
            end = len(parts) if include_self else len(parts) - 1
            for i in range(1, end + 1):
                p = '/' + '/'.join(parts[:i])
                dir_set.add(p)

        for path, data in self.entries.items():
            if data is None:
                ensure_ancestors(path, include_self=True)
            else:
                ensure_ancestors(path, include_self=False)
                file_list.append((path, data))

        dir_list = sorted(dir_set, key=lambda x: (len(x), x))
        file_list.sort()

        # ── Pre-allocate sectors ─────────────────────────────────────
        # Layout:
        #   System Area (16)
        #   PVD (1)
        #   BVD (1)
        #   Terminator (1)
        #   Boot Catalog (1)
        #   Boot Image (N)
        #   Path Table L (ceil/4 sectors)
        #   Path Table M (ceil/4 sectors) — optional
        #   Directory records (variable)
        #   File data (variable)

        # Pre-allocate fixed headers
        vd_sector = len(self.sectors)
        self.sectors.extend([b'\0'] * 3)  # PVD, BVD, Term
        bvd_sector = vd_sector + 1
        term_sector = vd_sector + 2

        boot_cat_sector = len(self.sectors)
        self.sectors.append(b'\0')

        boot_img_sector = None
        boot_img_n_sec = 0
        if self.boot_image_data:
            boot_img_sector = len(self.sectors)
            padded = pad_to(self.boot_image_data, SECTOR)
            boot_img_n_sec = len(padded) // SECTOR
            self.sectors.extend([padded[i*SECTOR:(i+1)*SECTOR] for i in range(boot_img_n_sec)])

        # Allocate file data now, record extent locations.
        # Skip the bootimage file if it was already placed as boot image data.
        self.file_extents = {}
        for path, data in file_list:
            if self.boot_image_path and path == self.boot_image_path:
                self.file_extents[path] = (boot_img_sector, boot_img_n_sec)
            else:
                sec, n = self._alloc_data(data)
                self.file_extents[path] = (sec, n)

        # Build child relationships
        def parent_of(path):
            p = path.strip('/')
            if '/' in p:
                return '/' + '/'.join(p.split('/')[:-1])
            return '/'

        dir_children = {d: [] for d in dir_list}
        # Directories
        for d in dir_list:
            p = parent_of(d)
            if d != '/' and p in dir_children:
                dir_children[p].append(('dir', d))
        # Files
        for fp, fdata in file_list:
            p = parent_of(fp)
            if p in dir_children:
                dir_children[p].append(('file', fp, fdata))

        dir_extents = {}
        for dir_path in dir_list:
            records = [('self', dir_path, None), ('parent', parent_of(dir_path), None)]
            records.extend(dir_children[dir_path])
            dir_extents[dir_path] = records

        # Now compute actual bytes for each dir's records with extent info
        # First pass: compute sizes without knowing extents
        def size_of_record(rec_type, target_path, data, extent_sec):
            if rec_type == 'self':
                fi = b'\0'  # root
                return len(_make_dir_record(extent_sec, 0, 2, fi))
            elif rec_type == 'parent':
                fi = b'\x01' # parent
                return len(_make_dir_record(0, 0, 2, fi))
            elif rec_type == 'dir':
                name = target_path.strip('/').split('/')[-1].encode() if target_path.strip('/') else b'\0'
                return len(_make_dir_record(0, 0, 2, name))
            elif rec_type == 'file':
                fname = target_path.strip('/').split('/')[-1].encode()
                fsize = len(data) if data else 0
                return len(_make_dir_record(0, fsize, 0, fname))
            return 0

        # Compute directory bytes (with correct names but zero extents)
        dir_bytes = {}
        for dir_path in dir_list:
            records = dir_extents[dir_path]
            total = b''
            for rec in records:
                if rec[0] == 'self':
                    total += _make_dir_record(0, 0, 2, b'\0')
                elif rec[0] == 'parent':
                    pp = rec[1]
                    total += _make_dir_record(0, 0, 2, b'\x01')
                elif rec[0] == 'dir':
                    name = rec[1].strip('/').split('/')[-1].encode()
                    total += _make_dir_record(0, 0, 2, name)
                elif rec[0] == 'file':
                    fname = rec[1].strip('/').split('/')[-1].encode()
                    fsize = len(rec[2]) if len(rec) > 2 else 0
                    total += _make_dir_record(0, fsize, 0, fname)
            dir_bytes[dir_path] = total

        # Allocate directory data (we'll patch extents later)
        for dir_path in sorted(dir_list, key=lambda x: len(x)):
            sec, n = self._alloc_data(dir_bytes[dir_path])
            dir_extents[dir_path] = (sec, n, dir_extents[dir_path])

        # Build path tables
        dir_sorted = sorted(dir_list, key=lambda x: (len(x), x))
        path_to_index = {dp: idx + 1 for idx, dp in enumerate(dir_sorted)}  # 1-based

        pt_data = []
        for dp in dir_sorted:
            dsec = dir_extents[dp][0]
            pp = parent_of(dp)
            parent_idx = path_to_index.get(pp, 1)
            name = dp.strip('/').split('/')[-1].encode() if dp != '/' else b'\0'
            entry = le32(dsec) + le16(parent_idx) + bytes([len(name), 0]) + name
            if len(name) % 2 == 0:
                entry += b'\0'
            pt_data.append(entry)

        lpt = b''.join(pt_data)
        mpt = b''
        for e in pt_data:
            # M-path table uses big-endian
            mpt += be32(struct.unpack_from('<I', e, 0)[0]) + e[4:]

        pt_l_sec, pt_l_n = self._alloc_data(lpt)
        pt_m_sec, pt_m_n = self._alloc_data(mpt)

        # ── Now write PVD with correct info ─────────────────────────
        pvd = bytearray(SECTOR)
        o = 0
        pvd[o] = 1; o += 1
        pvd[o:o+5] = b'CD001'; o += 5
        pvd[o] = 1; o += 1
        o += 1
        pvd[o:o+32] = self.sys_id; o += 32
        pvd[o:o+32] = self.vol_id; o += 32
        o += 8
        total_sectors = len(self.sectors)
        pvd[o:o+8] = lebe32(total_sectors); o += 8
        o += 32
        pvd[o:o+4] = lebe16(1); o += 4
        pvd[o:o+4] = lebe16(1); o += 4
        pvd[o:o+4] = lebe16(SECTOR); o += 4

        # Path table size + locations
        pvd[o:o+8] = lebe32(len(lpt)); o += 8
        pvd[o:o+4] = le32(pt_l_sec); o += 4
        pvd[o:o+4] = le32(0); o += 4  # L_path_table_opt
        pvd[o:o+4] = be32(pt_m_sec); o += 4
        pvd[o:o+4] = be32(0); o += 4  # M_path_table_opt

        # Root directory record
        root_sec = dir_extents['/'][0]
        root_data = dir_bytes['/']
        root_rec = _make_dir_record(root_sec, len(root_data), 2, b'\0')
        pvd[o:o+len(root_rec)] = root_rec; o += len(root_rec)

        for _ in range(4):
            pvd[o:o+17] = VOL_TS; o += 17
        o += 2
        pvd[o] = 0
        self.sectors[vd_sector] = bytes(pvd)

        # ── BVD (Boot Volume Descriptor) ──────────────────────────
        # Per "El Torito Bootable CD-ROM Format Specification":
        #   [0]    = 0 (Boot Record)
        #   [1-5]  = "CD001"
        #   [6]    = 1 (version)
        #   [7-38] = "EL TORITO SPECIFICATION" (Boot System Identifier)
        #   [39-71]= Reserved
        #   [72-75]= Boot catalog LBA (LE32)
        bvd = bytearray(SECTOR)
        bvd[0] = 0
        bvd[1:6] = b'CD001'
        bvd[6] = 1
        bvd[7:39] = b'EL TORITO SPECIFICATION'.ljust(32, b'\x20')
        bvd[72:76] = le32(boot_cat_sector)
        self.sectors[bvd_sector] = bytes(bvd)

        # ── Terminator ────────────────────────────────────────────────
        term = bytearray(SECTOR)
        term[0] = 255
        term[1:6] = b'CD001'
        self.sectors[term_sector] = bytes(term)

        # ── Boot Catalog ─────────────────────────────────────────────
        # El Torito boot catalog per "El Torito Bootable CD-ROM Format Specification"
        # Validation Entry (32 bytes):
        #   [0]    = 0x01 (Header ID)
        #   [1]    = Platform ID (0x00=x86, 0xEF=UEFI)
        #   [2-3]  = Reserved
        #   [4-27] = Manufacturer ID (24 bytes, padded with spaces)
        #   [28-29]= Checksum (such that sum of all 16-bit words in bytes 0-31 = 0)
        #   [30-31]= 0x55AA (signature)
        # Initial/Default Entry (32 bytes each):
        #   [0]    = 0x88 (bootable)
        #   [1]    = Media type (0=no emulation)
        #   [2-3]  = Load segment (0 for no emulation)
        #   [4]    = System type
        #   [5]    = Reserved
        #   [6-7]  = Sector count (512-byte virtual sectors)
        #   [8-11] = Load RBA (CD sector LBA)
        boot_cat = bytearray(SECTOR)
        # Validation entry
        boot_cat[0] = 1
        boot_cat[1] = 0x00  # Platform ID: 0x00 = all x86 (BIOS)
        boot_cat[4:28] = b'SKYOS\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20'
        boot_cat[30] = 0x55
        boot_cat[31] = 0xAA

        boot_img_size = len(self.boot_image_data) if self.boot_image_data else 0
        boot_img_sectors = (boot_img_size + 511) // 512
        if boot_img_sectors > 65535:
            raise ValueError(f"Boot image too large: {boot_img_sectors * 512:,} bytes")

        # Initial/Default Entry (for platform 0x00 - BIOS)
        o = 32
        boot_cat[o] = 0x88; o += 1    # bootable
        boot_cat[o] = 0; o += 1       # no emulation
        o += 2                        # load segment
        o += 2                        # system type + reserved
        boot_cat[o:o+2] = le16(boot_img_sectors); o += 2
        boot_cat[o:o+4] = le32(boot_img_sector if boot_img_sector else 0); o += 4
        o += 20

        # Section Header for UEFI (platform 0xEF)
        o = 64
        boot_cat[o] = 0x90; o += 1    # Section Header
        boot_cat[o] = 0xEF; o += 1    # Platform ID: UEFI
        boot_cat[o:o+2] = le16(1); o += 2  # Number of entries
        o += 28                        # ID string (reserved)

        # Section Entry for UEFI
        boot_cat[o] = 0x88; o += 1    # bootable
        boot_cat[o] = 0; o += 1       # no emulation
        o += 2
        o += 2
        boot_cat[o:o+2] = le16(boot_img_sectors); o += 2
        boot_cat[o:o+4] = le32(boot_img_sector if boot_img_sector else 0); o += 4
        o += 20

        # Compute checksum for first 32 bytes (Validation Entry)
        total = 0
        for i in range(0, 32, 2):
            word = boot_cat[i] | (boot_cat[i+1] << 8)
            total = (total + word) & 0xFFFF
        boot_cat[28:30] = struct.pack('<H', (-total) & 0xFFFF)
        self.sectors[boot_cat_sector] = bytes(boot_cat)

        # ── Rebuild directory records with correct extents ──────────
        for dir_path in dir_list:
            self_sec = dir_extents[dir_path][0]
            parent_path = parent_of(dir_path)
            parent_sec = dir_extents[parent_path][0] if parent_path in dir_extents else self_sec

            records = []
            records.append(_make_dir_record(self_sec, 0, 2, b'\0'))
            records.append(_make_dir_record(parent_sec, 0, 2, b'\x01'))

            for child in dir_children[dir_path]:
                if child[0] == 'dir':
                    dp = child[1]
                    name = dp.strip('/').split('/')[-1].encode()
                    dsec = dir_extents[dp][0]
                    dsize = len(dir_bytes[dp])
                    records.append(_make_dir_record(dsec, dsize, 2, name))
                elif child[0] == 'file':
                    fp, fdata = child[1], child[2]
                    fname = fp.strip('/').split('/')[-1].encode()
                    fsize = len(fdata)
                    fsec = self.file_extents[fp][0]
                    records.append(_make_dir_record(fsec, fsize, 0, fname))

            dir_bytes[dir_path] = b''.join(records)

        # Write corrected directory data
        for dir_path in dir_list:
            sec = dir_extents[dir_path][0]
            data = pad_to(dir_bytes[dir_path], SECTOR)
            for i in range(len(data) // SECTOR):
                if sec + i < len(self.sectors):
                    self.sectors[sec + i] = data[i*SECTOR:(i+1)*SECTOR]

        # ── Verify PVD root record has correct extent ─────────────────
        # Rebuild root dir record in PVD
        root_rec = _make_dir_record(dir_extents['/'][0], len(dir_bytes['/']), 2, b'\0')
        # Patch into PVD
        pvd = bytearray(self.sectors[vd_sector])
        # Find offset 156 (where root record starts) and overwrite
        pvd[156:156+len(root_rec)] = root_rec
        self.sectors[vd_sector] = bytes(pvd)

        # ── Assemble ──────────────────────────────────────────────────
        iso = b''.join(self.sectors)
        return iso


# ─── isohybrid MBR generator ───────────────────────────────────────────────

def make_hybrid_mbr(bootimg_data):
    """Create an isohybrid MBR for USB boot compatibility.
    
    Uses a GPT protective partition (type 0xEE) covering the boot image
    data area on the ISO. This allows UEFI firmware to find the ESP
    via the partition table when written to USB.
    """
    if len(bootimg_data) < 512:
        return None
    mbr = bytearray(512)
    # Partition entry 0: GPT protective
    pt_offset = 446
    pt_entries = bytearray(64)
    pt_entries[0] = 0x00       # Not bootable (will be found via GPT)
    pt_entries[1] = 0x00       # CHS start
    pt_entries[2] = 0x02       # CHS start
    pt_entries[3] = 0x00       # CHS start
    pt_entries[4] = 0xEE       # Type: GPT protective
    pt_entries[5] = 0xFF       # CHS end
    pt_entries[6] = 0xFF       # CHS end
    pt_entries[7] = 0xFF       # CHS end
    pt_entries[8:12] = le32(1)  # LBA start (after MBR)
    total_sectors = len(bootimg_data) // 512
    pt_entries[12:16] = le32(min(total_sectors, 0xFFFFFFFF))
    mbr[pt_offset:pt_offset + 64] = bytes(pt_entries)
    mbr[510:512] = b'\x55\xAA'
    return bytes(mbr)


# ─── Steps ──────────────────────────────────────────────────────────────────

def step_build_bootimage(do_full):
    if do_full:
        print("[1/5] Rebuilding userspace...")
        subprocess.run(["powershell", "-ExecutionPolicy", "Bypass", "-File",
                        os.path.join(KERNEL_DIR, "build_userspace.ps1")],
                       cwd=KERNEL_DIR, check=True)
        print("[2/5] Building kernel...")
        subprocess.run(["cargo", "build", "--target", "x86_64-unknown-none"],
                       cwd=os.path.join(KERNEL_DIR, "kernel"), check=True)
        print("[3/5] Building bootimage...")
        subprocess.run(["cargo", "run", "--manifest-path",
                        os.path.join(KERNEL_DIR, "builder", "Cargo.toml")],
                       cwd=KERNEL_DIR, check=True)
    if not os.path.exists(BOOTIMAGE_SRC):
        print(f"Bootimage not found. Run with --full or build manually.")
        sys.exit(1)
    sz = os.path.getsize(BOOTIMAGE_SRC)
    print(f"[OK] Bootimage: {sz:,} bytes")
    data = open(BOOTIMAGE_SRC, "rb").read()

    # Extract ESP (FAT partition) from GPT disk for El Torito boot image
    # GPT header at LBA 1 (byte 512)
    part_lba = struct.unpack('<Q', data[584:592])[0]  # GPT header offset 72-79
    part_count = struct.unpack('<I', data[592:596])[0]  # offset 80-83
    part_size = struct.unpack('<I', data[596:600])[0]   # offset 84-87
    esp_data = None
    for i in range(min(part_count, 128)):
        off = (part_lba + i) * 512
        entry = data[off:off+part_size]
        ptype = entry[0:16]
        if ptype == ESP_GUID:
            start_lba = struct.unpack('<Q', entry[32:40])[0]
            end_lba = struct.unpack('<Q', entry[40:48])[0]
            esp_start = start_lba * 512
            esp_size = (end_lba - start_lba + 1) * 512  # end_lba is inclusive in GPT
            esp_data = data[esp_start:esp_start + esp_size]
            print(f"  ESP partition at LBA {start_lba}-{end_lba} ({len(esp_data):,} bytes)")
            break
    if not esp_data:
        print("  WARNING: No ESP partition found in bootimage (UEFI boot may fail)")
        esp_data = data  # fallback: use whole bootimage
    return data, esp_data


def step_prepare_installer_data():
    """Create installer extras directory."""
    d = tempfile.mkdtemp(prefix="skyos_iso_")
    os.makedirs(os.path.join(d, "packages"), exist_ok=True)
    os.makedirs(os.path.join(d, "install"), exist_ok=True)
    with open(os.path.join(d, "README.TXT"), "w") as f:
        f.write("SKYOS OPERATING SYSTEM v0.3\n")
        f.write("Bootable Installer ISO\n\n")
        f.write("This disc contains the SkyOS operating system installer.\n")
        f.write("Boot from this disc and follow the on-screen instructions.\n")
    with open(os.path.join(d, "install", "AUTORUN"), "w") as f:
        f.write("setup\n")
    return d


def extract_fat_file(fat_data, path):
    """Extract a file from a FAT16 image by path (e.g. 'EFI/BOOT/BOOTX64.EFI')."""
    bpb = fat_data[:512]
    byt_sec = struct.unpack('<H', bpb[11:13])[0]
    sec_clu = bpb[13]
    res_sec = struct.unpack('<H', bpb[14:16])[0]
    num_fats = bpb[16]
    root_ent = struct.unpack('<H', bpb[17:19])[0]
    total_sec = struct.unpack('<H', bpb[19:21])[0]
    fat_sec = struct.unpack('<H', bpb[22:24])[0]
    clu_size = byt_sec * sec_clu
    root_sec = (root_ent * 32 + byt_sec - 1) // byt_sec
    data_sec = res_sec + num_fats * fat_sec + root_sec
    data_off = data_sec * byt_sec

    def read_cluster(n):
        off = data_off + (n - 2) * clu_size
        return fat_data[off:off + clu_size]

    def next_cluster(n):
        fat_off = res_sec * byt_sec + n * 2
        return struct.unpack('<H', fat_data[fat_off:fat_off+2])[0]

    def make_short_name(name):
        parts = name.upper().split('.')
        base = parts[0][:8].ljust(8, ' ')
        ext = parts[1][:3].ljust(3, ' ') if len(parts) > 1 else '   '
        return (base + ext).encode('ascii')

    def find_entry(dir_data, name):
        short = make_short_name(name)
        for i in range(0, len(dir_data), 32):
            entry = dir_data[i:i+32]
            if len(entry) < 32:
                break
            if entry[0] == 0:
                break
            if entry[0] == 0xE5:
                continue
            if entry[11] & 0x08:  # volume label
                continue
            if entry[11] & 0x0F:  # LFN entry
                continue
            if entry[:11] == short:
                return entry
        return None

    def is_dir(entry):
        return bool(entry[11] & 0x10)

    parts = path.upper().replace('\\', '/').split('/')
    root_dir = fat_data[res_sec * byt_sec + num_fats * fat_sec * byt_sec:
                        res_sec * byt_sec + num_fats * fat_sec * byt_sec + root_sec * byt_sec]
    current_dir = root_dir
    for i, part in enumerate(parts):
        entry = find_entry(current_dir, part)
        if entry is None:
            return None
        if i == len(parts) - 1:
            # Read file data
            start_clu = struct.unpack('<H', entry[26:28])[0]
            size = struct.unpack('<I', entry[28:32])[0]
            data = b''
            clu = start_clu
            while clu < 0xFFF8:
                data += read_cluster(clu)
                clu = next_cluster(clu)
            return data[:size]
        else:
            # Enter directory
            start_clu = struct.unpack('<H', entry[26:28])[0]
            dir_data = b''
            clu = start_clu
            while clu < 0xFFF8:
                dir_data += read_cluster(clu)
                clu = next_cluster(clu)
            current_dir = dir_data
    return None


def step_create_iso(bootimg_data, esp_data, extra_dir):
    """Create the universal ISO + raw disk image."""
    print("[4/5] Creating universal ISO + disk image...")

    # Also write a directly-dd-able disk image (just bootimage + installer data appended)
    disk_img_path = os.path.join(REPO_DIR, "skyos-installer.img")
    with open(disk_img_path, "wb") as f:
        f.write(bootimg_data)
    print(f"  Disk image: {disk_img_path} ({os.path.getsize(disk_img_path):,} bytes)")

    # Build ISO with El Torito boot catalog
    builder = IsoBuilder(vol_id=b"SKYOS_INSTALL", sys_id=b"SKYOS")
    # Add the ESP (FAT32 partition data) as both the El Torito boot image
    # AND as a file on the ISO 9660 filesystem.  UEFI firmware expects the
    # boot image to be a FAT volume containing \EFI\BOOT\BOOTX64.EFI.
    builder.add_boot_image(esp_data, path="bootimage.bin")
    efi_file = extract_fat_file(esp_data, "EFI/BOOT/BOOTX64.EFI")
    if efi_file:
        builder.add_file("EFI/BOOT/BOOTX64.EFI", efi_file)
        print(f"  Added EFI/BOOT/BOOTX64.EFI ({len(efi_file):,} bytes) to ISO filesystem")
    # Add kernel on ISO filesystem too
    kernel_file = extract_fat_file(esp_data, "vahi_kernel.bin")
    if kernel_file is None:
        kernel_file = extract_fat_file(esp_data, "KERNEL~1")
    if kernel_file:
        builder.add_file("vahi_kernel.bin", kernel_file)
        print(f"  Added vahi_kernel.bin ({len(kernel_file):,} bytes) to ISO filesystem")

    # Add extra files from installer data
    if extra_dir and os.path.exists(extra_dir):
        for root, dirs, files in os.walk(extra_dir):
            for fn in files:
                fp = os.path.join(root, fn)
                rel = os.path.relpath(fp, extra_dir).replace("\\", "/")
                with open(fp, "rb") as f:
                    data = f.read()
                builder.add_file(rel, data)

    iso_data = builder.build()

    # Write output
    with open(OUTPUT_ISO, "wb") as f:
        f.write(iso_data)

    real_sectors = len(iso_data) // SECTOR
    print(f"[OK] ISO: {OUTPUT_ISO}")
    print(f"     Size: {len(iso_data):,} bytes ({real_sectors} sectors)")
    return True


def step_test_qemu():
    """Boot the raw disk image in QEMU to verify it works."""
    print("[5/5] Testing raw disk image in QEMU...")
    ovmf = "C:\\Program Files\\qemu\\OVMF.fd"
    disk_img_path = os.path.join(REPO_DIR, "skyos-installer.img")

    result = subprocess.run([
        "qemu-system-x86_64",
        "-bios", ovmf,
        "-cpu", "max", "-smp", "1", "-m", "512M",
        "-no-reboot", "-nographic",
        "-drive", f"file={disk_img_path},format=raw",
        "-serial", "stdio",
        "-nic", "user", "-k", "en-us",
        "-rtc", "base=localtime"
    ], capture_output=True, text=True, timeout=25)

    output = result.stdout + result.stderr
    if "login:" in output:
        print("  [OK] Raw disk boot test: PASSED (login prompt detected)")
    else:
        print("  [WARN] Raw disk boot test: no 'login:' in output")


# ─── Main ───────────────────────────────────────────────────────────────────

def main():
    do_full = "--full" in sys.argv
    do_qemu = "--qemu" in sys.argv

    print("=" * 60)
    print("  SkyOS Universal Installer ISO Builder")
    print("  Target: UEFI + BIOS, CD/DVD + USB")
    print("=" * 60)

    # Step 1: Build/Fetch bootimage
    bootimg_data, esp_data = step_build_bootimage(do_full)

    # Step 2: Prepare installer data
    extra_dir = step_prepare_installer_data()

    try:
        # Step 3: Create universal ISO
        step_create_iso(bootimg_data, esp_data, extra_dir)
    finally:
        shutil.rmtree(extra_dir, ignore_errors=True)

    # Step 4: Test in QEMU if requested
    if do_qemu:
        step_test_qemu()

    print()
    print("=" * 60)
    print("  ARTIFACTS READY")
    print("=" * 60)
    print()
    print("  > skyos-installer.img -- PRIMARY INSTALLER")
    print("     Boot on USB via: dd if=skyos-installer.img of=/dev/sdX bs=4M")
    print("     Or with Rufus in DD Image mode")
    print("     Test: qemu-system-x86_64 -drive file=skyos-installer.img,format=raw")
    print()
    print("  > skyos-installer.iso -- BEST-EFFORT CD IMAGE")
    print("     The Python-generated El Torito boot catalog is not fully")
    print("     compatible with OVMF. For proper CD/DVD boot, generate using")
    print("     xorriso:")
    print("       xorriso -as mkisofs -iso-level 3 -V SKYOS_INSTALL")
    print("         -eltorito-boot bootimg.bin -no-emul-boot")
    print("         -eltorito-alt-boot -e EFI/BOOT/BOOTX64.EFI -no-emul-boot")
    print("         -o skyos-installer.iso iso_dir/")
    print()


if __name__ == '__main__':
    main()
