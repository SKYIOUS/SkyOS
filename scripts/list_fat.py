"""List files in the FAT16 ESP image."""
import struct, sys

esp_path = sys.argv[1] if len(sys.argv) > 1 else 'build/esp.img'
with open(esp_path, 'rb') as f:
    data = f.read(512)
    bps = struct.unpack('<H', data[11:13])[0]
    spc = data[13]
    resvd = struct.unpack('<H', data[14:16])[0]
    num_fats = data[16]
    root_ents = struct.unpack('<H', data[17:19])[0]
    sec_per_fat = struct.unpack('<H', data[22:24])[0]
    
    print(f'FAT16: BPB={bps} SPC={spc} Res={resvd} FATs={num_fats} Root={root_ents} SpF={sec_per_fat}')
    
    fat1_start = resvd * bps
    root_start = fat1_start + num_fats * sec_per_fat * bps
    data_start = root_start + root_ents * 32
    
    print(f'FAT1 at {fat1_start}  Root at {root_start}  Data at {data_start}')
    
    # Read root directory
    with open(esp_path, 'rb') as f:
        f.seek(root_start)
        root = f.read(root_ents * 32)
        
        def parse_dir(entries, dir_name='/'):
            for i in range(0, len(entries), 32):
                entry = entries[i:i+32]
                if entry[0] == 0:  # End of directory
                    break
                if entry[0] == 0xE5:  # Deleted
                    continue
                if entry[11] & 0x0F == 0x0F:  # LFN entry
                    continue
                name = entry[0:8].decode('ascii', errors='replace').rstrip(' ')
                ext = entry[8:11].decode('ascii', errors='replace').rstrip(' ')
                full = f'{name}.{ext}' if ext else name
                is_dir = bool(entry[11] & 0x10)
                filesize = struct.unpack('<I', entry[28:32])[0]
                first_clust = struct.unpack('<H', entry[26:28])[0]
                print(f'  {dir_name}{full:30s} {"(dir)" if is_dir else ""} cluster={first_clust} size={filesize}')
                
                if is_dir and name not in ('.', '..'):
                    # Read subdirectory
                    # First cluster data location
                    clust = first_clust
                    # Cluster 2 = first data cluster
                    if clust >= 2:
                        clust_off = data_start + (clust - 2) * spc * bps
                        f.seek(clust_off)
                        sub = f.read(spc * bps)
                        parse_dir(sub, f'{dir_name}{name}/')
        
        parse_dir(root)

    # Total ESP size
    import os
    print(f'\nTotal ESP file size: {os.path.getsize(esp_path):,} bytes')
