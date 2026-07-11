"""Debug tool: analyze ISO structure (MBR, GPT, El Torito)."""
import struct, zlib, sys

def analyze(path):
    with open(path, 'rb') as f:
        data = f.read()

    size = len(data)
    total_sectors = size // 512
    print(f'File: {size:,} bytes ({total_sectors} sectors of 512)')

    # MBR
    print('\n--- MBR ---')
    print(f'  sig: {data[510:512].hex()}')
    mbr_type = data[450]
    print(f'  partition type: {mbr_type:#04x}')
    print(f'  partition start: {struct.unpack_from("<I", data, 454)[0]}')
    print(f'  partition size: {struct.unpack_from("<I", data, 458)[0]}')

    # GPT header (at LBA 1 = offset 512)
    print('\n--- GPT header (LBA 1) ---')
    gpt = data[512:604]
    print(f'  sig:                {gpt[0:8]}')
    print(f'  rev:                {struct.unpack_from("<I", gpt, 8)[0]}')
    print(f'  hdr_size:           {struct.unpack_from("<I", gpt, 12)[0]}')
    hdr_crc = struct.unpack_from("<I", gpt, 16)[0]
    print(f'  hdr_crc_stored:     {hdr_crc:#010x}')
    print(f'  my_lba:             {struct.unpack_from("<Q", gpt, 24)[0]}')
    print(f'  last_lba:           {struct.unpack_from("<Q", gpt, 32)[0]}')
    print(f'  first_usable_lba:   {struct.unpack_from("<Q", gpt, 40)[0]}')
    print(f'  last_usable_lba:    {struct.unpack_from("<Q", gpt, 48)[0]}')
    print(f'  disk_guid:          {gpt[56:72].hex()}')
    print(f'  entries_start_lba:  {struct.unpack_from("<Q", gpt, 72)[0]}')
    print(f'  num_entries:        {struct.unpack_from("<I", gpt, 80)[0]}')
    print(f'  entry_size:         {struct.unpack_from("<I", gpt, 84)[0]}')

    # Verify header CRC
    tmp = bytearray(gpt)
    tmp[16:20] = b'\x00\x00\x00\x00'
    calc_crc = zlib.crc32(tmp) & 0xFFFFFFFF
    print(f'  hdr_crc_calc:       {calc_crc:#010x}  match={hdr_crc==calc_crc}')

    # Partition entries
    entries_start = struct.unpack_from("<Q", gpt, 72)[0]
    num_entries = struct.unpack_from("<I", gpt, 80)[0]
    entry_size = struct.unpack_from("<I", gpt, 84)[0]
    entries = data[entries_start * 512:entries_start * 512 + num_entries * entry_size]

    print(f'\n--- Partition Entry 0 (at LBA {entries_start}) ---')
    type_guid = entries[0:16]
    efi_guid = bytes.fromhex('28732AC11FF8D211BA4B00A0C93EC93B')
    print(f'  type_guid: {type_guid.hex()}  EFI_SP={type_guid==efi_guid}')
    unique_guid = entries[16:32]
    print(f'  unique_guid: {unique_guid.hex()}')
    start = struct.unpack_from("<Q", entries, 32)[0]
    end = struct.unpack_from("<Q", entries, 40)[0]
    print(f'  start_lba: {start}')
    print(f'  end_lba:   {end}')
    print(f'  size:      {end-start+1} sectors ({(end-start+1)*512:,} bytes)')
    name = entries[56:128].rstrip(b'\x00').decode('utf-16-le', errors='replace')
    print(f'  name:      {name}')

    # Verify entries CRC
    stored_entries_crc = struct.unpack_from("<I", gpt, 88)[0]
    calc_entries_crc = zlib.crc32(entries) & 0xFFFFFFFF
    print(f'  entries_crc_stored: {stored_entries_crc:#010x}')
    print(f'  entries_crc_calc:   {calc_entries_crc:#010x}')
    print(f'  entries_crc_match:  {stored_entries_crc == calc_entries_crc}')

    # Dump raw GPT header bytes
    print(f'\n--- Raw GPT header hex ---')
    for i in range(0, 92, 16):
        hex_str = ' '.join(f'{b:02x}' for b in gpt[i:i+16])
        print(f'  {i:3d}: {hex_str}')

    # El Torito Boot Record at LBA 17
    print('\n--- El Torito ---')
    boot_rec = data[17 * 2048:18 * 2048]
    print(f'  desc_type: {boot_rec[0]}')
    print(f'  std_id: {boot_rec[1:6]}')
    cat_ptr = struct.unpack_from("<I", boot_rec, 71)[0]
    print(f'  catalog_ptr: {cat_ptr} (LBA {cat_ptr})')

    # Validation Entry
    cat_offset = cat_ptr * 2048
    cat = data[cat_offset:cat_offset + 2048]
    val = cat[0:32]
    print(f'  validation entry: {val.hex()}')
    print(f'  platform: {val[0]:#04x}')
    print(f'  id_string: {val[28:30]}')
    checksum = struct.unpack_from("<H", val, 30)[0]
    print(f'  checksum: {checksum:#06x} ({checksum})')
    s = sum(val) & 0xFFFF
    print(f'  sum: {s:#06x}  valid={s==0}')

    # Initial Entry
    init = cat[32:64]
    print(f'  init entry: {init.hex()}')
    rba = struct.unpack_from("<I", init, 16)[0]
    sec_count = struct.unpack_from("<I", init, 12)[0]
    media_type = init[0] & 0x0F if len(init) > 1 else 0
    print(f'  media_type: {media_type}  RBA: {rba}  sect_count: {sec_count}')
    print(f'  ESP at LBA {rba * 4} (512B)')

    # Check ESP
    esp_lba = rba * 4
    if esp_lba > 0 and esp_lba < total_sectors:
        esp = data[esp_lba * 512:esp_lba * 512 + 512]
        print(f'  ESP sig: {esp[510:512].hex()}  OEM: {esp[3:11]}')
        has_bpb = esp[0] in (0xEB, 0xE9) and esp[510:512] == b'\x55\xAA'
        print(f'  BPB: {has_bpb}')
    else:
        print(f'  (RBA=0, no ESP to check)')

if __name__ == '__main__':
    analyze(sys.argv[1] if len(sys.argv) > 1 else 'release/skyos-0.6.0.iso')
