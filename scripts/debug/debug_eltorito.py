"""Debug pycdlib El Torito by creating a minimal ISO and examining boot catalog."""
import pycdlib, io, struct

# Create a minimal FAT image (a valid FAT12 boot sector + padding)
# We'll create just a boot sector with no actual files
fat_bootsector = bytearray(512)
fat_bootsector[0:3] = b'\xeb\x3c\x90'
fat_bootsector[3:11] = b'MSWIN4.1'
fat_bootsector[11:13] = struct.pack('<H', 512)  # bytes per sector
fat_bootsector[13] = 1  # sectors per cluster
fat_bootsector[14:16] = struct.pack('<H', 1)  # reserved sectors
fat_bootsector[16] = 1  # number of FATs
fat_bootsector[17:19] = struct.pack('<H', 224)  # root entries
fat_bootsector[19:21] = struct.pack('<H', 2880)  # total sectors (1.44M)
fat_bootsector[21] = 0xF0  # media descriptor
fat_bootsector[22:24] = struct.pack('<H', 9)  # sectors per FAT
fat_bootsector[510:512] = b'\x55\xaa'  # boot signature

# Also put EFI/BOOT/BOOTX64.EFI marker in the data area
fake_efi = b'\x4d\x5a\x00\x00'  # MZ header for bootloader.efi
fat_image = bytes(fat_bootsector) + b'\x00' * (2048 - 512)  # pad to 2048

iso_path = r'C:\Users\nanda\Desktop\Github\SkyOS\build\test_iso.iso'
esp_path = r'C:\Users\nanda\Desktop\Github\SkyOS\build\test_esp.bin'

with open(esp_path, 'wb') as f:
    f.write(fat_image)

iso = pycdlib.PyCdlib()
iso.new(interchange_level=3, vol_ident=b'TEST_ISO')

iso.add_file(esp_path, '/ESP.IMG;1')
iso.add_eltorito(
    '/ESP.IMG;1',
    boot_load_size=4,
    platform_id=0xef,
    efi=True,
    media_name='noemul',
    bootable=True,
)

iso.write(iso_path)
iso.close()

# Fix boot record (pycdlib bug)
with open(iso_path, 'r+b') as f:
    iso2 = pycdlib.PyCdlib()
    iso2.open(iso_path)
    boot_cat_lba = None
    for child in iso2.list_children(iso_path='/'):
        name = child.file_identifier().decode('ascii').split(';')[0]
        if name == 'BOOT.CAT':
            boot_cat_lba = child.extent_location()
            break
    iso2.close()
    
    if boot_cat_lba and boot_cat_lba != 0:
        f.seek(17 * 2048 + 39)
        f.write(struct.pack('<I', boot_cat_lba))
        f.write(struct.pack('>I', boot_cat_lba))
        print(f"Fixed boot catalog LBA to {boot_cat_lba}")

# Analyze the ISO
print("\n=== ISO Analysis ===")
with open(iso_path, 'rb') as f:
    # Volume descriptors
    for i in range(16, 20):
        f.seek(i * 2048)
        vd = f.read(7)
        print(f"Sector {i}: type={vd[0]}, id={vd[1:6]}, ver={vd[6]}")
        if vd[0] == 255:
            break
        if vd[0] == 0:  # Boot record
            f.seek(i * 2048 + 39)
            boot_id = f.read(8)
            lba = struct.unpack_from('<I', boot_id, 0)[0]
            print(f"  Boot catalog LBA: {lba}")
            # Read boot catalog
            f.seek(lba * 2048)
            cat = f.read(64)
            print(f"  Validation: header={cat[0]:02x}, platform={cat[1]:02x}, key={cat[31]:02x}")
            print(f"  Initial: indicator={cat[32]:02x}, media={cat[33]:02x}, load_rba={struct.unpack_from('<I', cat, 40)[0]}, sectors={struct.unpack_from('<H', cat, 38)[0]}")

# Test in QEMU
print("\nBooting test ISO in QEMU...")
import subprocess
import time

proc = subprocess.Popen(
    ['qemu-system-x86_64',
     '-bios', r'C:\Users\nanda\Desktop\Github\SkyOS\OVMF.fd',
     '-cdrom', iso_path,
     '-m', '512M', '-smp', '2',
     '-serial', 'stdio',
     '-display', 'none',
     '-no-reboot', '-no-shutdown'],
    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    cwd=r'C:\Users\nanda\Desktop\Github\SkyOS'
)

try:
    out, _ = proc.communicate(timeout=10)
    output = out.decode('utf-8', errors='replace')
    if 'failed to load' in output:
        print("EL TORITO: NOT FOUND")
    elif 'UEFI bootloader' in output or 'Booting' in output:
        print("EL TORITO: BOOT SUCCESSFUL!")
    else:
        print("EL TORITO: Unknown result")
    print(output[:500])
except subprocess.TimeoutExpired:
    proc.kill()
    print("QEMU timed out (which means it might be booting successfully!)")
except Exception as e:
    print(f"QEMU error: {e}")
