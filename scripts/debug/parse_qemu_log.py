"""parse_qemu_log.py — Extract and analyze QEMU serial output logs."""
import re, sys
from collections import Counter

def parse_log(path):
    with open(path, 'rb') as f:
        data = f.read()
    text = data.decode('utf-8', errors='replace')
    lines = text.split('\n')
    print(f"File: {path}")
    print(f"Lines: {len(lines)}")
    print(f"Size: {len(data)} bytes")
    print()
    # Count tagged lines
    tags = Counter()
    untagged = 0
    for line in lines:
        m = re.match(r'\[([A-Z_/]+)\]', line.strip())
        if m:
            tags[m.group(1)] += 1
        elif line.strip():
            untagged += 1
    print("=== Tag Frequency ===")
    for t, c in tags.most_common():
        print(f"  [{t}] {c}")
    if untagged:
        print(f"  (untagged) {untagged}")
    print()
    # Extract panics / errors
    print("=== Panics / Errors ===")
    panics = [l for l in lines if 'PANIC' in l.upper() or 'panic' in l.lower()]
    for p in panics:
        print(f"  {p.strip()}")
    if not panics:
        print("  (none)")
    print()
    # Extract boot markers
    print("=== Boot Timeline ===")
    boots = [l for l in lines if re.match(r'\[(BOOT|SPLASH|init|IRQ|TRACE)\]', l.strip())]
    for b in boots:
        print(f"  {b.strip()}")
    print()
    # Timing info (last lines)
    print("=== Last 10 Lines ===")
    for l in lines[-10:]:
        print(f"  {l.strip()}")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        # Try default log locations
        import glob
        logs = (glob.glob("*.log") + glob.glob("../SKYIOUS KERNEL/*.log")
                + glob.glob("../SKYIOUS KERNEL/tests/*.txt"))
        if logs:
            print("No log specified, using latest:")
            latest = max(logs, key=os.path.getmtime)
            print(f"  {latest}\n")
            parse_log(latest)
        else:
            print("Usage: python parse_qemu_log.py <logfile>")
    else:
        parse_log(sys.argv[1])
