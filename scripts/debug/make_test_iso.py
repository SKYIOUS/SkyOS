"""Create minimal test ISO for El Torito debug."""
import pycdlib, struct, os

esp = bytearray(512)
esp[0:3] = b'\xeb\x3c\x90'
esp[3:11] = b'MSWIN4.1'
esp[11:13] = struct.pack('<H', 512)
esp[13] = 1
esp[14:16] = struct.pack('<H', 1)
esp[16] = 1
esp[17:19] = struct.pack('<H', 224)
esp[19:21] = struct.pack('<H', 2880)
esp[21] = 0xf0
esp[22:24] = struct.pack('<H', 9)
esp[510:512] = b'\x55\xaa'

esp_path = r'C:\Users\nanda\Desktop\Github\SkyOS\build\test_esp.bin'
with open(esp_path, 'wb') as f:
    f.write(bytes(esp) + b'\x00' * (2048 - 512))

iso = pycdlib.PyCdlib()
iso.new(interchange_level=3, vol_ident='TEST_ISO')
iso.add_file(esp_path, '/ESP.IMG;1')
iso.add_eltorito('/ESP.IMG;1', boot_load_size=4, platform_id=0xef,
                 efi=True, media_name='noemul', bootable=True)
iso_path = r'C:\Users\nanda\Desktop\Github\SkyOS\build\test.iso'
iso.write(iso_path)
iso.close()

# Fix boot record LBA
with open(iso_path, 'r+b') as f:
    iso2 = pycdlib.PyCdlib()
    iso2.open(iso_path)
    lba = None
    for c in iso2.list_children(iso_path='/'):
        n = c.file_identifier().decode('ascii').split(';')[0]
        if n == 'BOOT.CAT':
            lba = c.extent_location()
            break
    iso2.close()
    f.seek(17 * 2048 + 39)
    f.write(struct.pack('<I', lba) + struct.pack('>I', lba))
    print(f'Fixed boot catalog LBA={lba}')

# Verify
with open(iso_path, 'rb') as f:
    f.seek(17 * 2048 + 39)
    d = f.read(4)
    lba_val = struct.unpack('<I', d)[0]
    print(f'Boot catalog LBA in record: {lba_val}')
    f.seek(lba_val * 2048)
    c = f.read(64)
    print(f'Validation: hdr={c[0]:02x} plat={c[1]:02x} key={c[31]:02x}')
    load_rba = struct.unpack('<I', c[40:44])[0]
    sectors = struct.unpack('<H', c[38:40])[0]
    print(f'Initial: ind={c[32]:02x} media={c[33]:02x} lba={load_rba} sectors={sectors}')

print(f'ISO size: {os.path.getsize(iso_path)} bytes')
print('ISO ready for QEMU test')
