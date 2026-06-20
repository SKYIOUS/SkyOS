"""
Minimal ISO 9660 writer with El Torito UEFI boot support.

Writes raw ISO structures directly without pycdlib to avoid its bugs.
"""
import struct, os, sys, io

SECTOR_SIZE = 2048

def pad_to(data, size):
    """Pad data with zeros to given size."""
    return data + b'\x00' * (size - len(data))

def make_volume_descriptor(vd_type, std_id=b'CD001', ver=1, data=b''):
    """Create a volume descriptor header + data."""
    return struct.pack('<B5sB', vd_type, std_id, ver) + data

def make_pvd(vol_ident=b'SARGA_OS'):
    """Create Primary Volume Descriptor."""
    pvd = b''
    # Skip unused bytes (we fill with zeros for minimal implementation)
    # Volume descriptor type, CD001, version handled by caller
    pvd += b'\x00'  # unused
    pvd += struct.pack('<I', 0)  # system identifier (32 bytes) - we'll use bytes
    pvd += pad_to(vol_ident, 32)  # volume identifier
    pvd += b'\x00' * 2  # unused
    pvd += struct.pack('<I', 0)  # volume space size (set later)
    pvd += b'\x00' * 4
    pvd += struct.pack('<H', 1)  # volume set size
    pvd += struct.pack('<H', 1)  # volume sequence number
    pvd += struct.pack('<H', 2048)  # logical block size
    pvd += struct.pack('<Q', 0)  # path table size (set later)
    return None  # Too complex to implement from scratch

# Actually this approach is too complex. Let me try to fix the actual issue.

def create_iso_manually(esp_data, output_path, version='0.6.0'):
    """Create ISO with raw structures."""
    import struct, os
    vol_name = (f'SARGA_OS_{version}').encode('ascii')[:32]
    
    # Calculate layout
    # We need: system area (16 sectors) + volume descriptors + files
    # For simplicity: 
    #   Sector 0-15: System Area (zeros)
    #   Sector 16: PVD
    #   Sector 17: Boot Record (type 0)
    #   Sector 18: Terminator
    #   Sector 19-23: Free/path tables
    #   Sector 24: Root Directory
    #   Sector 25: Boot Catalog (BOOT.CAT)
    #   Sector 26+: ESP.IMG
    #   Last sector: README
    
    esp_size = len(esp_data)
    esp_sectors = (esp_size + SECTOR_SIZE - 1) // SECTOR_SIZE
    esp_lba = 26  # Start sector for ESP.IMG
    esp_end_lba = esp_lba + esp_sectors
    
    readme_content = f'SARGA OS {version} - UEFI Bootable Installer\n'
    readme_data = readme_content.encode('ascii')
    readme_lba = esp_end_lba
    readme_sectors = (len(readme_data) + SECTOR_SIZE - 1) // SECTOR_SIZE
    
    total_sectors = readme_lba + readme_sectors
    total_size = total_sectors * SECTOR_SIZE
    volume_space_size = total_size  # in bytes
    
    # Build the ISO
    with open(output_path, 'wb') as f:
        # 1. System Area (16 sectors = 32768 bytes)
        f.write(b'\x00' * 16 * SECTOR_SIZE)
        
        # 2. PVD at sector 16
        # Volume descriptor type 1 (PVD), ID "CD001", version 1
        pvd = bytearray(SECTOR_SIZE)
        pvd[0] = 1  # type
        pvd[1:6] = b'CD001'
        pvd[6] = 1  # version
        pvd[7] = 0  # unused
        # bytes 8-39: system identifier (32 bytes)
        # bytes 40-71: volume identifier (32 bytes)
        pvd[40:40+len(vol_name)] = vol_name
        # bytes 72-79: unused
        # bytes 80-87: volume space size (LE + BE)
        # Actually ISO 9660 stores both LE and BE for numeric values
        # But for simplicity, we'll use a minimal correct PVD
        
        # Simplified PVD - just enough for a working ISO
        # Actually, this is getting very complex. ISO 9660 has many fields.
        
        # Write the minimal correct PVD structure
        f.seek(16 * SECTOR_SIZE)
        f.write(b'\x01')  # type 1 = PVD
        f.write(b'CD001')
        f.write(b'\x01')  # version 1
        f.write(b'\x00')  # unused
        f.write(b'\x00' * 32)  # system identifier (empty)
        f.write(vol_name.ljust(32, b'\x00'))  # volume identifier
        f.write(b'\x00' * 8)  # unused
        # volume space size (8 bytes LE, 8 bytes BE) - starts at offset 80
        # LE: 4 bytes LE, 4 bytes zero
        # BE: 4 bytes BE, 4 bytes zero
        f.write(struct.pack('<I', total_sectors) + b'\x00' * 4)  # LE volume space size
        f.write(struct.pack('>I', total_sectors) + b'\x00' * 4)  # BE volume space size
        
        # ... this is getting extremely long and complex
        
        f.write(pad_to(b'', SECTOR_SIZE))
        
        # This approach is impractical. Let me stop here.
        pass

    print("Manual ISO creation is too complex to implement correctly from scratch.")
    return False

# Let's go back to fixing pycdlib-based approach
