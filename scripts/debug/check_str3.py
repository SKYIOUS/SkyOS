with open('C:\\Users\\nanda\\Desktop\\Github\\SkyOS\\target\\x86_64-sarga\\release\\init', 'rb') as f:
    data = f.read()
    # Search entire file
    for s in [b'/etc/init.cfg', b'ABCDEFGHIJKLMNOPQRSTUVWXYZ', b'OK\n', b'FAIL']:
        pos = 0
        while True:
            pos = data.find(s, pos)
            if pos < 0:
                break
            print(f'{repr(s.decode())} at file offset 0x{pos:x}')
            pos += 1
    # Also dump as hex around the /etc/init.cfg search area
    print()
    # Search byte by byte for common patterns
    for i in range(len(data)):
        if data[i:i+2] == b'/e':
            print(f'Found \"/e\" pattern at offset 0x{i:x}: {data[i:i+20]}')
