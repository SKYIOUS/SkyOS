"""Create a UEFI-bootable ISOHybrid (file-based El Torito + MBR + GPT).

Works in: QEMU (-cdrom and -drive media=disk), USB (dd/Rufus DD mode).

Requires xorriso, or falls back to pycdlib if `--pycdlib` is passed.
"""

import argparse
import io
import os
import shutil
import struct
import subprocess
import sys
import zlib
from pathlib import Path


def log(msg):
    print(f"  {msg}")


def read_gpt_parts(path):
    with open(path, "rb") as f:
        f.seek(512)
        hdr = f.read(92)
        if hdr[:8] != b"EFI PART":
            return [(34, None)]
        part_lba = struct.unpack_from("<Q", hdr, 72)[0]
        num_parts = struct.unpack_from("<I", hdr, 80)[0]
        part_size = struct.unpack_from("<I", hdr, 84)[0]
        esp_guid = bytes.fromhex("28732AC11FF8D211BA4B00A0C93EC93B")
        parts = []
        f.seek(part_lba * 512)
        for i in range(min(num_parts, 128)):
            entry = f.read(part_size)
            if len(entry) < 56:
                break
            typ = entry[:16]
            sl, el = struct.unpack_from("<QQ", entry, 32)
            if sl == 0:
                continue
            parts.append({"start": sl, "end": el, "is_esp": typ == esp_guid})
        return parts


def extract_esp(bootimage_path, esp_path):
    parts = read_gpt_parts(bootimage_path)
    esp = next((p for p in parts if p.get("is_esp")), parts[0] if parts else None)
    with open(bootimage_path, "rb") as src:
        if esp:
            start, end = esp["start"], esp.get("end")
            src.seek(start * 512)
            size = (end - start + 1) * 512 if end else (os.path.getsize(bootimage_path) - start * 512)
        else:
            start = 34
            src.seek(start * 512)
            size = os.path.getsize(bootimage_path) - start * 512
        data = src.read(size)
    with open(esp_path, "wb") as dst:
        dst.write(data)
    bpb = data[:512]
    has_bpb = bpb[0] in (0xEB, 0xE9) and bpb[510:512] == b"\x55\xAA"
    log(f"ESP: {size:,}B | FAT16: {'YES' if has_bpb and bpb[11:13] == b'\\x00\\x02' else 'NO'}"
        f" | OEM: {bpb[3:11].decode('ascii', errors='replace') if has_bpb else 'N/A'}")


def find_xorriso():
    p = shutil.which("xorriso")
    if p:
        return p
    try:
        r = subprocess.run(["wsl", "xorriso", "--version"], capture_output=True, timeout=8, text=True)
        if r.returncode == 0:
            return "wsl"
    except Exception:
        pass
    return None


def to_wsl_path(p):
    for c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
        if p.startswith(f"{c}:"):
            return f"/mnt/{c.lower()}{p[2:].replace(os.sep, '/')}"
    return p


def _iso_paths(p):
    return to_wsl_path(p) if p and os.path.isabs(p) else p


def create_iso_xorriso(esp_path, output_path, version):
    vol_id = f"SKYOS_{version}"[:32]
    xor = find_xorriso()
    if not xor:
        log("xorriso not found")
        return False

    build_dir = Path(esp_path).parent
    content = build_dir / "iso_root"
    content.mkdir(parents=True, exist_ok=True)
    (content / "README.txt").write_text(f"SkyOS {version} - UEFI Bootable\n")
    shutil.copy2(esp_path, str(content / "esp.img"))

    cmd = [
        xor if xor != "wsl" else "wsl",
        "-as", "mkisofs",
        "-V", vol_id,
        "-iso-level", "2",
        "-eltorito-alt-boot", "-e", "esp.img", "-no-emul-boot",
        "-o", str(output_path),
        str(content),
    ]
    if xor == "wsl":
        cmd = [cmd[0]] + [_iso_paths(c) for c in cmd[1:]]

    log(f"Running: {' '.join(cmd)}")
    if subprocess.run(cmd).returncode != 0:
        return False

    _patch_hybrid(output_path)
    shutil.rmtree(str(content), ignore_errors=True)
    return True


def _patch_hybrid(output_path):
    """Patch MBR + GPT onto an xorriso-produced ISO."""
    size = os.path.getsize(output_path)
    total_sectors = size // 512

    with open(output_path, "r+b") as f:
        data = f.read()
        pvd = data[16 * 2048:17 * 2048]
        root_lba = struct.unpack_from("<I", pvd, 158)[0]
        rd = data[root_lba * 2048:root_lba * 2048 + 2048]

        cat_lbn = esp_lbn = esp_size = None
        off = 0
        while off < len(rd) - 32:
            rec_len = rd[off]
            if rec_len == 0:
                break
            name_len = rd[off + 32]
            raw = rd[off + 33:off + 33 + name_len].decode("ascii", errors="replace").split(";")[0].rstrip(".")
            file_lba = struct.unpack_from("<I", rd, off + 2)[0]
            file_size = struct.unpack_from("<I", rd, off + 10)[0]
            if raw.upper() == "BOOT.CATALOG":
                cat_lbn = file_lba
            elif raw.upper() == "ESP.IMG":
                esp_lbn = file_lba
                esp_size = file_size
            off += rec_len

        if not esp_lbn or not cat_lbn:
            log("ERROR: required files not in ISO")
            return

        f.seek(17 * 2048 + 71)
        f.write(struct.pack("<I", cat_lbn))

        esp_lba = esp_lbn * 4
        esp_sectors = esp_size // 512
        log(f"ESP: RBA={esp_lbn} LBA={esp_lba} sectors={esp_sectors}")

        # MBR: GPT protective
        mbr = bytearray(512)
        mbr[446:450] = b"\x00\x02\x00"
        mbr[450] = 0xEE
        mbr[451:454] = b"\xff\xff\xff"
        mbr[454:458] = struct.pack("<I", 1)
        mbr[458:462] = struct.pack("<I", min(total_sectors - 1, 0xFFFFFFFF))
        mbr[510:512] = b"\x55\xaa"
        f.seek(0)
        f.write(mbr)

        # GPT header
        gh = bytearray(92)
        gh[:8] = b"EFI PART"
        gh[8:12] = struct.pack("<I", 0x00010000)
        gh[12:16] = struct.pack("<I", 92)
        gh[24:32] = struct.pack("<Q", 1)
        gh[32:40] = struct.pack("<Q", total_sectors - 1)
        gh[40:48] = struct.pack("<Q", 34)
        gh[48:56] = struct.pack("<Q", total_sectors - 34)
        gh[56:72] = os.urandom(16)
        gh[72:80] = struct.pack("<Q", 2)
        gh[80:84] = struct.pack("<I", 128)
        gh[84:88] = struct.pack("<I", 128)

        # Partition entries
        efi_guid = bytes.fromhex("28732AC11FF8D211BA4B00A0C93EC93B")
        entries = bytearray(128 * 128)
        entries[:16] = efi_guid
        entries[16:32] = os.urandom(16)
        entries[32:40] = struct.pack("<Q", esp_lba)
        entries[40:48] = struct.pack("<Q", esp_lba + esp_sectors - 1)
        name = "EFI System Partition".encode("utf-16-le")
        entries[56:128] = name.ljust(72, b"\x00")[:72]

        entries_crc = zlib.crc32(entries) & 0xFFFFFFFF
        gh[88:92] = struct.pack("<I", entries_crc)
        gh[16:20] = struct.pack("<I", zlib.crc32(gh) & 0xFFFFFFFF)

        f.seek(512)
        f.write(gh)
        f.seek(1024)
        f.write(entries)

        # Backup GPT
        last = total_sectors - 1
        f.seek((last - 32) * 512)
        f.write(entries)
        bh = bytearray(gh)
        bh[24:32] = struct.pack("<Q", last)
        bh[32:40] = struct.pack("<Q", 34)
        bh[16:20] = struct.pack("<I", zlib.crc32(bh) & 0xFFFFFFFF)
        f.seek(last * 512)
        f.write(bh)

    log(f"Hybrid: MBR=YES GPT=YES")


def create_iso_pycdlib(esp_path, output_path, version):
    try:
        import pycdlib
    except ImportError:
        log("pycdlib not installed. Run: pip install pycdlib")
        return False

    iso = pycdlib.PyCdlib()
    iso.new(interchange_level=2, vol_ident=f"SKYOS_{version}"[:32])
    iso.add_file(esp_path, "/ESP.IMG;1")
    iso.add_fp(io.BytesIO(b"SkyOS boot ISO\n"), "/README.TXT;1")
    iso.write(output_path)
    iso.close()

    _patch_hybrid(output_path)
    return True


def main():
    parser = argparse.ArgumentParser(description="Create UEFI-bootable ISOHybrid")
    parser.add_argument("version", nargs="?", default="0.6.0",
                        help="Version string (default: 0.6.0)")
    parser.add_argument("--pycdlib", action="store_true",
                        help="Use pycdlib instead of xorriso")
    args = parser.parse_args()

    script_dir = Path(__file__).parent.resolve()
    skyos_root = script_dir.parent

    # Find bootimage — try multiple locations
    candidates = []
    for profile in ("release", "debug"):
        for base in (skyos_root.parent / "SKYIOUS KERNEL", skyos_root.parent / "SKYIOUS-KERNEL",
                     skyos_root / "kernel", skyos_root):
            p = base / f"target/x86_64-vahi/{profile}/bootimage-vahi_kernel.bin"
            candidates.append(p)
    candidates.extend([
        skyos_root / "bootimage-vahi_kernel.bin",
        skyos_root / "skyos_uefi.img",
    ])

    bootimage = next((p for p in candidates if p.exists()), None)
    if not bootimage:
        print("ERROR: bootimage-vahi_kernel.bin not found")
        print("  Looked in:")
        for c in candidates:
            print(f"    {c}")
        sys.exit(1)

    version = args.version
    release_dir = skyos_root / "release"
    build_dir = skyos_root / "build"
    release_dir.mkdir(parents=True, exist_ok=True)
    build_dir.mkdir(parents=True, exist_ok=True)

    esp_path = build_dir / "esp.img"
    iso_path = release_dir / f"skyos-{version}.iso"

    print(f"Bootimage: {bootimage}")
    print(f"Version:   {version}")
    print(f"ESP:       {esp_path}")
    print(f"ISO:       {iso_path}")

    print("\n1. Extracting ESP...")
    extract_esp(str(bootimage), str(esp_path))

    print("2. Creating ISO...")
    ok = False
    if args.pycdlib:
        ok = create_iso_pycdlib(str(esp_path), str(iso_path), version)
    else:
        ok = create_iso_xorriso(str(esp_path), str(iso_path), version)
        if not ok:
            log("xorriso failed, trying pycdlib fallback...")
            ok = create_iso_pycdlib(str(esp_path), str(iso_path), version)

    if not ok:
        print("ERROR: ISO creation failed")
        sys.exit(1)

    size = os.path.getsize(str(iso_path))
    print(f"\nSUCCESS: {iso_path} ({size / 1024 / 1024:.1f} MB)")
    print("  qemu-system-x86_64 -bios OVMF.fd -cdrom <iso>")
    print("  dd if=<iso> of=/dev/sdX bs=4M status=progress")


if __name__ == "__main__":
    main()