"""Create a UEFI-bootable hybrid ISO from the bootimage's EFI System Partition.
Uses xorriso for reliable El Torito UEFI boot support."""
import struct, os, sys, subprocess

def read_gpt_partitions(image_path):
    with open(image_path, 'rb') as f:
        f.seek(512)
        hdr = f.read(92)
        sig = hdr[0:8]
        if sig != b'EFI PART':
            return [(34, None)]
        part_start = struct.unpack_from('<Q', hdr, 72)[0]
        num_parts  = struct.unpack_from('<I', hdr, 80)[0]
        part_size  = struct.unpack_from('<I', hdr, 84)[0]
        efi_guid = bytes.fromhex('28732AC11FF8D211BA4B00A0C93EC93B')
        parts = []
        f.seek(part_start * 512)
        for i in range(min(num_parts, 128)):
            entry = f.read(part_size)
            if len(entry) < 56:
                break
            type_guid = entry[0:16]
            start_lba, end_lba, attrs = struct.unpack_from('<QQQ', entry, 32)
            if start_lba == 0 and end_lba == 0:
                continue
            parts.append({
                'index': i, 'start_lba': start_lba, 'end_lba': end_lba,
                'is_esp': (type_guid == efi_guid),
            })
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
        with open(esp_path, 'wb') as dst:
            dst.write(src.read(size))
    print(f"  ESP extracted: {size} bytes (LBA {start})")

def find_xorriso():
    """Find xorriso executable."""
    import shutil
    # Check PATH first
    path = shutil.which('xorriso')
    if path:
        return path
    # Check WSL
    try:
        subprocess.run(['wsl', 'which', 'xorriso'], capture_output=True, timeout=5)
        return 'wsl'
    except:
        pass
    # Check common locations on Windows
    for p in [
        r'C:\Program Files\xorriso\bin\xorriso.exe',
        r'C:\Program Files (x86)\xorriso\bin\xorriso.exe',
    ]:
        if os.path.exists(p):
            return p
    return None

def run_xorriso(args):
    """Run xorriso, handling WSL wrapper if needed."""
    xorriso = find_xorriso()
    if not xorriso:
        return False
    if xorriso == 'wsl':
        wsl_path = lambda p: p.replace('\\', '/').replace('C:', '/mnt/c').replace('D:', '/mnt/d')
        wsl_args = ['wsl'] + args
        wsl_args = [arg if arg.startswith('-') else wsl_path(arg) for arg in args]
        # Prepend xorriso command
        cmd = ['wsl', 'xorriso'] + args[1:] if args[0] == 'xorriso' else ['wsl'] + args
        result = subprocess.run(cmd, capture_output=False)
    else:
        result = subprocess.run(args, capture_output=False)
    return result.returncode == 0

def create_iso(esp_path, output_path, version):
    """Create hybrid UEFI-bootable ISO using xorriso."""
    content_dir = os.path.dirname(esp_path)
    esp_name = os.path.basename(esp_path)
    vol_id = f'SARGA_OS_{version}'.upper().replace('-', '_')[:32]
    
    # Use xorriso to create hybrid ISO with UEFI boot
    cmd = [
        'xorriso', '-as', 'mkisofs',
        '-V', vol_id,
        '-iso-level', '3',
        '-full-iso9660-filenames',
        '-eltorito-alt-boot',
        '-e', esp_name,
        '-no-emul-boot',
        '-isohybrid-gpt-basdat',
        '-o', output_path,
        content_dir,
    ]
    
    xorriso = find_xorriso()
    if xorriso == 'wsl':
        # Convert Windows paths to WSL paths
        wsl_map = {}
        for d in ['C:', 'D:', 'E:', 'F:']:
            wsl_map[d] = f'/mnt/{d[0].lower()}'
        def to_wsl(p):
            for d, m in wsl_map.items():
                if p.startswith(d):
                    return m + p[2:].replace('\\', '/')
            return p
        cmd = ['wsl'] + ['xorriso'] + cmd[1:]
        cmd = [to_wsl(c) if not c.startswith('-') else c for c in cmd]
    
    print(f"  Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, check=False)
    if result.returncode != 0:
        print("  xorriso failed, falling back to pycdlib")
        return create_iso_pycdlib(esp_path, output_path, version)
    
    print(f"  ISO created: {os.path.getsize(output_path)} bytes")
    return True

def create_iso_pycdlib(esp_path, output_path, version):
    """Fallback ISO creation using pycdlib."""
    import io, pycdlib
    iso = pycdlib.PyCdlib()
    iso.new(interchange_level=3, vol_ident=f'SARGA_OS_{version}')
    iso.add_file(esp_path, '/ESP.IMG;1')
    iso.add_eltorito('/ESP.IMG;1', boot_load_size=16, platform_id=0xef,
                     efi=True, media_name='noemul', bootable=True)
    readme = f"SARGA OS {version} - UEFI Bootable Installer\n"
    bio = io.BytesIO(readme.encode('utf-8'))
    iso.add_fp(bio, len(readme), '/README.TXT;1')
    iso.write(output_path)
    iso.close()
    # Fix boot record LBA (pycdlib bug)
    with open(output_path, 'r+b') as f:
        iso2 = pycdlib.PyCdlib()
        iso2.open(output_path)
        boot_cat_lba = None
        for c in iso2.list_children(iso_path='/'):
            n = c.file_identifier().decode('ascii').split(';')[0]
            if n == 'BOOT.CAT':
                boot_cat_lba = c.extent_location()
                break
        iso2.close()
        if boot_cat_lba:
            for offset in [17 * 2048 + 39, 17 * 2048 + 71]:
                f.seek(offset)
                f.write(struct.pack('<I', boot_cat_lba))
    print(f"  ISO created (pycdlib fallback): {os.path.getsize(output_path)} bytes")
    return True

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    skyos_root = os.path.dirname(script_dir)
    
    # Search for bootimage in common layouts:
    #   local:     ../SKYIOUS KERNEL/target/...   (sibling repos)
    #   local2:    ../SKYIOUS-KERNEL/target/...   (sibling with hyphen)
    #   CI:        SKYIOUS-KERNEL/target/...      (subdir of workspace)
    profiles = ['release', 'debug']
    search_layouts = []
    for kdir in ['SKYIOUS KERNEL', 'SKYIOUS-KERNEL']:
        for profile in profiles:
            search_layouts.append(('parent', kdir, profile))
            search_layouts.append(('sibling', kdir, profile))
    bootimage = None
    for layout in search_layouts:
        rel_type, kdir, profile = layout
        if rel_type == 'parent':
            base = os.path.join(skyos_root, '..', kdir, 'target', 'x86_64-vahi', profile)
        else:
            base = os.path.join(skyos_root, kdir, 'target', 'x86_64-vahi', profile)
        p = os.path.join(base, 'bootimage-vahi_kernel.bin')
        if os.path.exists(p):
            bootimage = p
            break
    if not bootimage:
        for kdir in ['SKYIOUS KERNEL', 'SKYIOUS-KERNEL']:
            p = os.path.join(skyos_root, '..', kdir, 'bootimage-vahi_kernel.bin')
            if os.path.exists(p):
                bootimage = p
                break
            p = os.path.join(skyos_root, kdir, 'bootimage-vahi_kernel.bin')
            if os.path.exists(p):
                bootimage = p
                break
    if not bootimage:
        print("ERROR: bootimage-vahi_kernel.bin not found")
        sys.exit(1)
    
    version = sys.argv[1] if len(sys.argv) > 1 else '0.6.0'
    release_dir = os.path.join(skyos_root, 'release')
    build_dir  = os.path.join(skyos_root, 'build')
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
    create_iso(esp_path, iso_path, version)
    
    size = os.path.getsize(iso_path)
    print(f"\nSUCCESS: {iso_path} ({size / 1024 / 1024:.1f} MB)")

if __name__ == '__main__':
    main()
