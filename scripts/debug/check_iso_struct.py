"""Check ISO structure."""
with open(r'C:\Users\nanda\Desktop\Github\SkyOS\release\skyos-0.6.0.iso', 'rb') as f:
    # Check system sectors 0-15
    f.seek(0)
    for i in range(16):
        f.seek(i * 2048)
        s = f.read(2048)
        non_zero = sum(1 for b in s if b != 0)
        if non_zero > 0:
            print(f'Sector {i}: {non_zero} non-zero bytes')

    # PVD at sector 16
    f.seek(16 * 2048)
    pvd = f.read(2048)
    print(f'\nVolume descriptor at sector 16: type={pvd[0]}, id={pvd[1:6]}')
    vol_name = pvd[40:72].decode('ascii').rstrip(' ')
    print(f'Volume name: [{vol_name}]')

    # Check volume descriptors 16-19
    for i in range(16, 20):
        f.seek(i * 2048)
        vd = f.read(2048)
        if vd[0] == 0:  # Boot record
            std_id = vd[8:31].decode('ascii', errors='replace')
            boot_cat_lba = int.from_bytes(vd[40:44], 'little')
            print(f'\nBoot Record at sector {i}:')
            print(f'  Boot system ID: [{std_id}]')
            print(f'  Boot catalog LBA: {boot_cat_lba}')
            # Read boot catalog
            f.seek(boot_cat_lba * 2048)
            cat = f.read(2048)
            print(f'  Boot catalog validation: header={cat[0]:02x}, platform={cat[1]:02x}, key={cat[31]:02x}')
            print(f'  Initial entry: boot_ind={cat[32]:02x}, media={cat[33]:02x}, load_rba={int.from_bytes(cat[40:44], "little")}, sec_count={int.from_bytes(cat[38:40], "little")}')
            break
        elif vd[0] == 255:
            print(f'Volume descriptor set terminator at sector {i}')
            break
        else:
            print(f'Volume descriptor at sector {i}: type={vd[0]}, id={vd[1:6]}')
