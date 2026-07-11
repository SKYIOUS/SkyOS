"""Check El Torito boot catalog in ISO."""
import pycdlib, struct

iso = pycdlib.PyCdlib()
iso.open(r'C:\Users\nanda\Desktop\Github\SkyOS\release\skyos-0.6.0.iso')

# Walk all files to find BOOT.CAT
print('=== ISO contents ===')
for child in iso.list_children(iso_path='/'):
    print(f'  {child.file_identifier().decode("ascii")}')

# Read the boot catalog file directly
import io
buf = io.BytesIO()
iso.get_file_from_iso_fp(buf, iso_path='/BOOT.CAT;1')
cat_data = buf.getvalue()

print(f'\n=== El Torito Boot Catalog ({len(cat_data)} bytes) ===')

# Validation Entry (first 32 bytes)
print(f'  Validation Entry:')
print(f'    Header ID:    {cat_data[0]:02x}')
print(f'    Platform ID:  {cat_data[1]:02x}  ({"UEFI" if cat_data[1]==0xef else "x86" if cat_data[1]==0 else "PPC" if cat_data[1]==1 else "Mac" if cat_data[1]==2 else "Unknown"})')
print(f'    Reserved:     {cat_data[2]:02x} {cat_data[3]:02x}')
print(f'    ID String:    {cat_data[4:28].decode("ascii", errors="replace")}')
# Checksum
checksum = struct.unpack_from('<H', cat_data, 28)[0]
print(f'    Checksum:     {checksum:04x}')
# Key byte
print(f'    Key (byte 31): {cat_data[31]:02x}')

# Initial/Default Entry (second 32 bytes)  
print(f'\n  Initial/Default Entry:')
entry = cat_data[32:64]
boot_indicator = entry[0]
print(f'    Boot Indicator:           {boot_indicator:02x}  ({"Bootable" if boot_indicator==0x88 else "Not bootable"})')
boot_media_type = entry[1]
media_types = {0: 'No Emulation', 1: '1.2M floppy', 2: '1.44M floppy', 3: '2.88M floppy', 4: 'Hard disk'}
print(f'    Media Type:               {boot_media_type:02x}  ({media_types.get(boot_media_type, "Unknown")})')
load_segment = struct.unpack_from('<H', entry, 2)[0]
print(f'    Load Segment:             {load_segment:04x}')
system_type = entry[4]
print(f'    System Type:              {system_type:02x}')
# reserved
sector_count = struct.unpack_from('<H', entry, 6)[0]
print(f'    Sector Count:             {sector_count}')
load_rba = struct.unpack_from('<I', entry, 8)[0]
print(f'    Load RBA (sector):        {load_rba}')
# Also check selection criteria (bytes 12-19)
sel_crit = entry[12:20]
print(f'    Selection Criteria (12-19): {sel_crit.hex()}')

# Check if there's a second entry (for legacy BIOS)
if len(cat_data) > 64:
    entry2 = cat_data[64:96]
    print(f'\n  Section Entry 2:')
    boot_indicator2 = entry2[0]
    print(f'    Boot Indicator:           {boot_indicator2:02x}')
    boot_media_type2 = entry2[1]
    print(f'    Media Type:               {boot_media_type2:02x}  ({media_types.get(boot_media_type2, "Unknown")})')
    sel_crit2 = entry2[12:20]
    print(f'    Selection Criteria (12-19): {sel_crit2.hex()}')

iso.close()
