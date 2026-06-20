"""Check contiguity of ESP.IMG in ISO."""
import pycdlib, struct

iso = pycdlib.PyCdlib()
iso.open(r'C:\Users\nanda\Desktop\Github\SkyOS\release\skyos-0.6.0.iso')

# Get info about ESP.IMG from ISO 9660 directory record
# list_children returns directory records
for child in iso.list_children(iso_path='/'):
    name = child.file_identifier().decode('ascii').rstrip(';')
    print(f'Found file: [{name}]')
    if name == 'ESP.IMG' or name == 'ESP.IMG;1':
        # From the directory record, get extent location and data length
        # In ISO 9660, the extent location is at byte 2 of the directory record
        extent_loc = child.extent_location()
        data_len = child.get_data_length()
        print(f'ESP.IMG:')
        print(f'  Extent location (LBA): {extent_loc}')
        print(f'  Data length (bytes):   {data_len}')
        print(f'  Data length (sectors): {data_len // 2048}')
        print(f'  End sector (excl):     {extent_loc + (data_len + 2047) // 2048}')
        break

iso.close()

# Now check if the data at extent_loc matches ESP.IMG
with open(r'C:\Users\nanda\Desktop\Github\SkyOS\release\skyos-0.6.0.iso', 'rb') as f:
    f.seek(extent_loc * 2048)
    iso_esp_data = f.read(min(data_len, 512))

with open(r'C:\Users\nanda\Desktop\Github\SkyOS\build\esp.img', 'rb') as f:
    raw_esp_data = f.read(512)

print(f'\nFirst 16 bytes match: {iso_esp_data[:16] == raw_esp_data[:16]}')
print(f'ISO ESP first 16: {iso_esp_data[:16].hex()}')
print(f'Raw ESP first 16: {raw_esp_data[:16].hex()}')
