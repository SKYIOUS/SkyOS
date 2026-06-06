import struct
with open('C:\\Users\\nanda\\Desktop\\Github\\SkyOS\\target\\x86_64-sarga\\release\\init', 'rb') as f:
    data = f.read()
    strings = [b'Starting', b'/etc/init', b'Hello', b'SkyOS', b'cfg', b'FAIL', b'OK\n']
    for s in strings:
        pos = data.find(s)
        if pos >= 0:
            print(f'Found "{s.decode()}" at file offset 0x{pos:x}')
        else:
            print(f'"{s.decode()}" NOT found')
    # Find all readable strings
    for i in range(len(data) - 4):
        if data[i:i+4].isalnum() or all(32 <= c < 127 for c in data[i:i+4]):
            end = i
            while end < len(data) and (data[end] >= 32 and data[end] < 127):
                end += 1
            if end - i >= 6:
                s = data[i:end].decode('ascii', errors='replace')
                print(f'  str at 0x{i:x}: "{s}"')
                i = end
