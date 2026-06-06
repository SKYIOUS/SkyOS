with open('C:\\Users\\nanda\\Desktop\\Github\\SkyOS\\target\\x86_64-sarga\\release\\init', 'rb') as f:
    data = f.read()
    # Dump segment 2: offset 0x3468, size 0x604
    seg2 = data[0x3468:0x3468+0x604]
    print('Segment 2 data (read-only):')
    # Show printable ASCII strings
    i = 0
    while i < len(seg2):
        if 32 <= seg2[i] < 127:
            start = i
            while i < len(seg2) and 32 <= seg2[i] < 127:
                i += 1
            s = seg2[start:i].decode('ascii')
            if len(s) >= 4:
                va = 0x402468 + start
                print(f'  VA 0x{va:x}: {repr(s)}')
        else:
            i += 1
    print()
    # Also check if /etc/init.cfg exists anywhere
    target = b'/etc/init.cfg'
    if target in data:
        print(f'/etc/init.cfg found!')
    else:
        print('/etc/init.cfg NOT FOUND in binary!')
    # And look for the word "SkyOS"
    target2 = b'SkyOS'
    if target2 in data:
        print(f'SkyOS found at offset 0x{data.find(target2):x}')
    target3 = b'Starting'
    if target3 in data:
        print(f'Starting found at offset 0x{data.find(target3):x}')
