"""Check OVMF boot image size load limit.
Tests if OVMF can load small (<1MB) El Torito boot images."""
import struct, os, shutil, subprocess

# Create a minimal 1MB FAT16 image with just BOOTX64.EFI
build_dir = 'build'
esp_mini = os.path.join(build_dir, 'esp_mini.img')
bootx64 = os.path.join(build_dir, 'BOOTX64.EFI')

if not os.path.exists(bootx64):
    print("BOOTX64.EFI not found, extract first")
    exit(1)

# Create a 1MB file filled with zeros
img_size = 1 * 1024 * 1024
with open(esp_mini, 'wb') as f:
    f.write(b'\x00' * img_size)

# Format as FAT12 (which EFI supports) using FAT16 tools
# Actually, let's use a simpler approach: copy the existing esp.img and 
# check if trimming it down helps

# Try with a 1MB trimmed version of the full ESP
with open('build/esp.img', 'rb') as f:
    full_esp = f.read()

# The first 1MB contains FAT16 header (BPB, FAT tables, root dir) and part of data
# Write it out as a 1MB file
with open(esp_mini, 'wb') as f:
    f.write(full_esp[:img_size])

# Now test this in QEMU
iso_mini = os.path.join(build_dir, 'skyos-mini.iso')
iso_root = os.path.join(build_dir, 'iso_root_mini')
os.makedirs(iso_root, exist_ok=True)
shutil.copy2(esp_mini, os.path.join(iso_root, 'esp.img'))
with open(os.path.join(iso_root, 'README.txt'), 'w') as f:
    f.write('Test minimal boot\n')

# Use xorriso to create a simple ISO with this mini ESP
cmd = ['wsl', 'xorriso', '-as', 'mkisofs',
       '-V', 'TEST_BOOT',
       '-iso-level', '3',
       '-full-iso9660-filenames',
       '-eltorito-alt-boot',
       '-e', 'esp.img',
       '-no-emul-boot',
       '-o', to_wsl_path(iso_mini),
       to_wsl_path(iso_root)]

print(f'Running: {" ".join(cmd)}')
subprocess.run(cmd, check=True)

print(f'\nCreated mini ISO: {os.path.getsize(iso_mini)} bytes')

def to_wsl_path(p):
    p = os.path.abspath(p)
    for c in 'ABCDEFGHIJKLMNOPQRSTUVWXYZ':
        d = f'{c}:'
        if p.startswith(d):
            return '/mnt/' + c.lower() + p[2:].replace('\\', '/')
    return p
